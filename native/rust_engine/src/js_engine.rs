use crate::html_parser::{
    find_body, find_element_by_id, get_node_inner_html, get_node_text_content, query_selector,
    query_selector_all, remove_node, set_node_inner_html, set_node_text_content, HTMLParser,
    NodeData, NodePtr, NodeType,
};
use crate::network::URL;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, Once};
use std::thread;
use std::time::{Duration, Instant};
use v8;

static V8_INIT: Once = Once::new();

fn ensure_v8_initialized() {
    V8_INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

// ============================================================================
// ASYNC FETCH QUEUE & STRUCTS (PURE RUST DATA - NO V8 HANDLES ACROSS THREADS)
// ============================================================================

static NEXT_FETCH_ID: AtomicU64 = AtomicU64::new(1);
static HAS_PENDING_FETCH_RESULTS: AtomicBool = AtomicBool::new(false);
static FETCH_RESULT_QUEUE: Mutex<Option<Vec<FetchResult>>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub engine_id: usize,
    pub request_id: u64,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub url: String,
    pub body: String,
    pub error: Option<String>,
}

fn push_fetch_result(result: FetchResult) {
    let mut lock = FETCH_RESULT_QUEUE.lock().unwrap();
    if lock.is_none() {
        *lock = Some(Vec::new());
    }
    if let Some(ref mut q) = *lock {
        q.push(result);
    }
    HAS_PENDING_FETCH_RESULTS.store(true, Ordering::SeqCst);
}

fn drain_fetch_results_for_engine(engine_id: usize) -> Vec<FetchResult> {
    let mut lock = FETCH_RESULT_QUEUE.lock().unwrap();
    if let Some(ref mut q) = *lock {
        let (matching, remaining): (Vec<_>, Vec<_>) = q.drain(..).partition(|r| r.engine_id == engine_id);
        *q = remaining;
        if q.is_empty() {
            HAS_PENDING_FETCH_RESULTS.store(false, Ordering::SeqCst);
        }
        matching
    } else {
        Vec::new()
    }
}

// ============================================================================
// ACTIVE DOM & RUNTIME EXECUTION CONTEXT
// ============================================================================

struct ActiveContext {
    dom_root: Option<NodePtr>,
    current_url: String,
    dom_mutated: bool,
    is_parsing: bool,
    written_html_buffer: String,
    console_logs: Vec<String>,
    node_registry: HashMap<usize, NodePtr>,
    next_node_id: usize,
}

thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<ActiveContext>> = RefCell::new(None);
    static PENDING_TIMERS: RefCell<Vec<TimerTask>> = RefCell::new(Vec::new());
    static CANCELLED_TIMERS: RefCell<Vec<u32>> = RefCell::new(Vec::new());
    static NEXT_TIMER_ID: RefCell<u32> = RefCell::new(1);
    // V8 PromiseResolvers strictly retained on V8 isolate thread!
    static PENDING_FETCH_RESOLVERS: RefCell<HashMap<u64, v8::Global<v8::PromiseResolver>>> = RefCell::new(HashMap::new());
    // Origin-scoped persistent web storage and cookie jars
    static LOCAL_STORAGE: RefCell<HashMap<String, HashMap<String, String>>> = RefCell::new(HashMap::new());
    static SESSION_STORAGE: RefCell<HashMap<String, HashMap<String, String>>> = RefCell::new(HashMap::new());
    static COOKIE_JAR: RefCell<HashMap<String, Vec<(String, String)>>> = RefCell::new(HashMap::new());
}

// Timer Task for the Browser Event Loop
pub struct TimerTask {
    pub id: u32,
    pub delay: Duration,
    pub created_at: Instant,
    pub is_interval: bool,
    pub callback: v8::Global<v8::Function>,
}

pub struct V8JSEngine {
    pub engine_id: usize,
    isolate: Option<v8::OwnedIsolate>,
    page_context: Option<v8::Global<v8::Context>>,
    timers: Vec<TimerTask>,
}

impl V8JSEngine {
    pub fn new() -> Self {
        ensure_v8_initialized();

        let isolate = v8::Isolate::new(Default::default());

        Self {
            engine_id: 0,
            isolate: Some(isolate),
            page_context: None,
            timers: Vec::new(),
        }
    }

    /// Reset and establish a SINGLE PERSISTENT V8 CONTEXT for a new page load
    pub fn reset_page_context(&mut self, dom_root: Option<&NodePtr>, url_str: &str) {
        let isolate = match self.isolate.as_mut() {
            Some(iso) => iso,
            None => return,
        };

        // 1. Clear old timers, cancelled timer lists, and pending fetch resolvers
        self.timers.clear();
        PENDING_TIMERS.with(|q| q.borrow_mut().clear());
        CANCELLED_TIMERS.with(|c| c.borrow_mut().clear());
        PENDING_FETCH_RESOLVERS.with(|map| map.borrow_mut().clear());

        // 2. Set up Thread-Local Node Registry
        CURRENT_CONTEXT.with(|ctx| {
            let mut reg = HashMap::new();
            let mut next_id = 1;
            if let Some(root) = dom_root {
                register_dom_tree(root, &mut reg, &mut next_id);
            }

            *ctx.borrow_mut() = Some(ActiveContext {
                dom_root: dom_root.map(Rc::clone),
                current_url: url_str.to_string(),
                dom_mutated: false,
                is_parsing: false,
                written_html_buffer: String::new(),
                console_logs: Vec::new(),
                node_registry: reg,
                next_node_id: next_id,
            });
        });

        // 3. Create ONE persistent V8 Context for the page
        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(handle_scope, Default::default());
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let global = context.global(scope);

        // 4. Setup Global APIs & W3C DOM Bindings
        setup_window_global(scope, global);
        setup_console_api(scope, global);
        setup_location_api(scope, global, url_str);
        setup_navigator_api(scope, global);
        setup_document_api(scope, global);
        setup_timer_apis(scope, global);
        setup_async_fetch_api(scope, global, url_str, self.engine_id);
        setup_storage_and_cookies_api(scope, global, url_str);

        // 5. Inject DOM & EventTarget Prototype Helpers
        inject_dom_prototype_bootstrap(scope);

        // 6. Store persistent Context reference
        self.page_context = Some(v8::Global::new(handle_scope, context));
    }

    /// Execute a JavaScript snippet inside the CURRENT PERSISTENT PAGE CONTEXT with TryCatch exception reporting
    pub fn execute(&mut self, source: &str) -> Result<bool, String> {
        let isolate = self.isolate.as_mut().ok_or("V8 Isolate not available")?;
        let page_context_global = self
            .page_context
            .as_ref()
            .ok_or("No active page context. Call reset_page_context first.")?;

        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Local::new(handle_scope, page_context_global);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        // Setup TryCatch block to accurately capture JS exceptions
        let try_catch = &mut v8::TryCatch::new(scope);

        let code = v8::String::new(try_catch, source).ok_or("Failed to allocate JS source string")?;

        let script = match v8::Script::compile(try_catch, code, None) {
            Some(s) => s,
            None => {
                if let Some(exception) = try_catch.exception() {
                    let msg = exception.to_rust_string_lossy(try_catch);
                    let line = try_catch
                        .message()
                        .and_then(|m| m.get_line_number(try_catch))
                        .unwrap_or(0);
                    eprintln!("[Axomai V8 Compile Error @ line {}]: {}", line, msg);
                }
                return Err("V8 Compilation failed".to_string());
            }
        };

        let result = script.run(try_catch);
        if result.is_none() {
            if let Some(exception) = try_catch.exception() {
                let msg = exception.to_rust_string_lossy(try_catch);
                let line = try_catch
                    .message()
                    .and_then(|m| m.get_line_number(try_catch))
                    .unwrap_or(0);
                eprintln!("[Axomai V8 Uncaught Exception @ line {}]: {}", line, msg);
                if let Some(stack) = try_catch.stack_trace(try_catch) {
                    eprintln!("Stack Trace:\n{}", stack.to_rust_string_lossy(try_catch));
                }
            }
        }

        // 1. Perform Microtask Checkpoint so Promises (Promise.resolve, .then, async/await) resolve!
        try_catch.perform_microtask_checkpoint();

        // 2. Harvest any timers registered during script execution
        PENDING_TIMERS.with(|q| {
            self.timers.append(&mut *q.borrow_mut());
        });

        // 3. Filter out any cancelled timers
        CANCELLED_TIMERS.with(|c| {
            let cancelled = c.borrow();
            self.timers.retain(|t| !cancelled.contains(&t.id));
        });

        let was_mutated = CURRENT_CONTEXT.with(|ctx| {
            if let Some(ref mut c) = *ctx.borrow_mut() {
                let m = c.dom_mutated;
                c.dom_mutated = false;
                m
            } else {
                false
            }
        });

        Ok(was_mutated)
    }

    /// Process queued timers, async network fetch results, and microtasks (Browser Event Loop tick)
    pub fn process_event_loop(&mut self) -> bool {
        let isolate = match self.isolate.as_mut() {
            Some(iso) => iso,
            None => return false,
        };
        let page_context_global = match self.page_context.as_ref() {
            Some(ctx) => ctx,
            None => return false,
        };

        let now = Instant::now();
        let mut executed_any = false;

        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Local::new(handle_scope, page_context_global);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        // 0. Process Compositor Animation Frame Callbacks (requestAnimationFrame)
        let flush_raf_js = "if (typeof window !== 'undefined' && typeof window.__flushAnimationFrameCallbacks === 'function') { window.__flushAnimationFrameCallbacks(performance.now()); } else { false; }";
        if let Some(code) = v8::String::new(scope, flush_raf_js) {
            if let Some(script) = v8::Script::compile(scope, code, None) {
                if let Some(val) = script.run(scope) {
                    if val.is_true() {
                        executed_any = true;
                    }
                }
                scope.perform_microtask_checkpoint();
            }
        }

        // 1. Process Completed Async Fetch Requests (Non-blocking network thread)
        let completed_fetches = drain_fetch_results_for_engine(self.engine_id);

        for fetch_res in &completed_fetches {
            // Process Set-Cookie response headers into origin COOKIE_JAR
            for (k, v) in &fetch_res.headers {
                if k.eq_ignore_ascii_case("set-cookie") {
                    let parts: Vec<&str> = v.split(';').collect();
                    if let Some(first) = parts.first() {
                        let kv: Vec<&str> = first.splitn(2, '=').collect();
                        if kv.len() == 2 {
                            let ck = kv[0].trim().to_string();
                            let cv = kv[1].trim().to_string();
                            let orig = get_origin_from_url(&fetch_res.url);
                            COOKIE_JAR.with(|jar| {
                                let mut map = jar.borrow_mut();
                                let list = map.entry(orig).or_insert_with(Vec::new);
                                list.retain(|(item_k, _)| item_k != &ck);
                                let is_deleted = parts.iter().skip(1).any(|p| {
                                    let p_lower = p.trim().to_lowercase();
                                    p_lower == "max-age=0" || p_lower.starts_with("expires=thu, 01 jan 1970")
                                });
                                if !is_deleted {
                                    list.push((ck, cv));
                                }
                            });
                        }
                    }
                }
            }
        }

        for fetch_res in completed_fetches {
            let resolver_opt = PENDING_FETCH_RESOLVERS.with(|map| {
                map.borrow_mut().remove(&fetch_res.request_id)
            });

            if let Some(resolver_global) = resolver_opt {
                let resolver = resolver_global.open(scope);

                if let Some(err_msg) = fetch_res.error {
                    let err_v = v8::String::new(scope, &err_msg).unwrap();
                    resolver.reject(scope, err_v.into());
                } else {
                    let resp_obj = v8::Object::new(scope);

                    let status_k = v8::String::new(scope, "status").unwrap();
                    let status_v = v8::Integer::new(scope, fetch_res.status as i32);
                    resp_obj.set(scope, status_k.into(), status_v.into());

                    let status_text_k = v8::String::new(scope, "statusText").unwrap();
                    let status_text_v = v8::String::new(scope, &fetch_res.status_text).unwrap();
                    resp_obj.set(scope, status_text_k.into(), status_text_v.into());

                    let url_k = v8::String::new(scope, "url").unwrap();
                    let url_v = v8::String::new(scope, &fetch_res.url).unwrap();
                    resp_obj.set(scope, url_k.into(), url_v.into());

                    let ok_k = v8::String::new(scope, "ok").unwrap();
                    let ok_v = v8::Boolean::new(scope, fetch_res.status >= 200 && fetch_res.status < 300);
                    resp_obj.set(scope, ok_k.into(), ok_v.into());

                    // Headers object with get(name)
                    let headers_obj = v8::Object::new(scope);
                    let headers_map = v8::Object::new(scope);
                    for (k, v) in &fetch_res.headers {
                        let hk = v8::String::new(scope, &k.to_lowercase()).unwrap();
                        let hv = v8::String::new(scope, v).unwrap();
                        headers_map.set(scope, hk.into(), hv.into());
                    }
                    let map_local = headers_map.into();
                    let get_k = v8::String::new(scope, "get").unwrap();
                    let get_fn = v8::Function::new(
                        scope,
                        move |s: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut r: v8::ReturnValue| {
                            if args.length() > 0 {
                                let key = args.get(0).to_rust_string_lossy(s).to_lowercase();
                                let k_str = v8::String::new(s, &key).unwrap();
                                if let Some(obj) = map_local.to_object(s) {
                                    if let Some(val) = obj.get(s, k_str.into()) {
                                        if !val.is_undefined() {
                                            r.set(val);
                                            return;
                                        }
                                    }
                                }
                            }
                            r.set(v8::null(s).into());
                        },
                    )
                    .unwrap();
                    headers_obj.set(scope, get_k.into(), get_fn.into());

                    let headers_k = v8::String::new(scope, "headers").unwrap();
                    resp_obj.set(scope, headers_k.into(), headers_obj.into());

                    // text() method
                    let text_k = v8::String::new(scope, "text").unwrap();
                    let body_clone = fetch_res.body.clone();
                    let text_fn = v8::Function::new(
                        scope,
                        move |s: &mut v8::HandleScope,
                              _a: v8::FunctionCallbackArguments,
                              mut r: v8::ReturnValue| {
                            let text_res = v8::PromiseResolver::new(s).unwrap();
                            let text_p = text_res.get_promise(s);
                            let text_str = v8::String::new(s, &body_clone).unwrap();
                            text_res.resolve(s, text_str.into());
                            r.set(text_p.into());
                        },
                    )
                    .unwrap();
                    resp_obj.set(scope, text_k.into(), text_fn.into());

                    // json() method
                    let json_k = v8::String::new(scope, "json").unwrap();
                    let body_json = fetch_res.body.clone();
                    let json_fn = v8::Function::new(
                        scope,
                        move |s: &mut v8::HandleScope,
                              _a: v8::FunctionCallbackArguments,
                              mut r: v8::ReturnValue| {
                            let json_res = v8::PromiseResolver::new(s).unwrap();
                            let json_p = json_res.get_promise(s);
                            let json_str = v8::String::new(s, &body_json).unwrap();
                            if let Some(parsed) = v8::json::parse(s, json_str) {
                                json_res.resolve(s, parsed);
                            } else {
                                let err = v8::String::new(s, "Invalid JSON in response").unwrap();
                                json_res.reject(s, err.into());
                            }
                            r.set(json_p.into());
                        },
                    )
                    .unwrap();
                    resp_obj.set(scope, json_k.into(), json_fn.into());

                    resolver.resolve(scope, resp_obj.into());
                }

                executed_any = true;
                scope.perform_microtask_checkpoint();
            }
        }

        // 2. Process Scheduled Async Timers
        let mut remaining = Vec::new();

        for timer in self.timers.drain(..) {
            let is_cancelled = CANCELLED_TIMERS.with(|c| c.borrow().contains(&timer.id));
            if is_cancelled {
                continue;
            }

            if now.duration_since(timer.created_at) >= timer.delay {
                let func = timer.callback.open(scope);
                let recv = context.global(scope).into();

                let try_catch = &mut v8::TryCatch::new(scope);
                let _ = func.call(try_catch, recv, &[]);

                if let Some(exception) = try_catch.exception() {
                    let msg = exception.to_rust_string_lossy(try_catch);
                    eprintln!("[Axomai V8 Timer Exception]: {}", msg);
                }

                try_catch.perform_microtask_checkpoint();
                executed_any = true;

                if timer.is_interval {
                    remaining.push(TimerTask {
                        id: timer.id,
                        delay: timer.delay,
                        created_at: Instant::now(),
                        is_interval: true,
                        callback: timer.callback,
                    });
                }
            } else {
                remaining.push(timer);
            }
        }

        self.timers = remaining;

        // Harvest any newly queued timers from callbacks
        PENDING_TIMERS.with(|q| {
            self.timers.append(&mut *q.borrow_mut());
        });

        // Drain any pending microtasks
        scope.perform_microtask_checkpoint();

        let was_mutated = CURRENT_CONTEXT.with(|ctx| {
            if let Some(ref mut c) = *ctx.borrow_mut() {
                let m = c.dom_mutated;
                c.dom_mutated = false;
                m
            } else {
                false
            }
        });

        executed_any || was_mutated
    }

    /// Check whether there are active background fetch results or pending timers needing processing
    pub fn has_pending_events(&self) -> bool {
        HAS_PENDING_FETCH_RESULTS.load(Ordering::SeqCst)
            || PENDING_TIMERS.with(|t| !t.borrow().is_empty())
            || !self.timers.is_empty()
    }

    /// Dispatch a native click event with full Event object and bubbling
    pub fn dispatch_click_event(&mut self, target_selector: &str, click_x: f32, click_y: f32) -> bool {
        self.dispatch_pointer_event("click", target_selector, click_x, click_y, 0)
    }

    /// Dispatch pointer/mouse events (pointerdown, pointerup, pointermove, mousedown, mouseup, mousemove, click) with true event bubbling and capture routing
    pub fn dispatch_pointer_event(
        &mut self,
        event_type: &str,
        target_selector: &str,
        client_x: f32,
        client_y: f32,
        button: i32,
    ) -> bool {
        let js = format!(
            r#"
            (function() {{
                let el = null;
                const captured = (window.__capturedPointerElements && window.__capturedPointerElements.get(1));
                if (captured) {{
                    el = captured;
                }} else {{
                    el = document.querySelector("{sel}") || document.getElementById("{sel}") || document.body;
                }}

                if (!el) return false;

                const opts = {{
                    clientX: {x},
                    clientY: {y},
                    button: {btn},
                    buttons: {btns},
                    bubbles: true,
                    cancelable: true,
                    pointerId: 1,
                    pointerType: 'mouse',
                    isPrimary: true
                }};

                const evType = "{ev}";

                if (evType === 'pointermove') {{
                    const lastEl = window.__lastHoveredElement;
                    if (lastEl !== el) {{
                        if (lastEl) {{
                            if (typeof PointerEvent === 'function') {{
                                lastEl.dispatchEvent(new PointerEvent('pointerout', opts));
                                lastEl.dispatchEvent(new PointerEvent('pointerleave', Object.assign({{}}, opts, {{ bubbles: false }})));
                            }}
                            if (typeof MouseEvent === 'function') {{
                                lastEl.dispatchEvent(new MouseEvent('mouseout', opts));
                                lastEl.dispatchEvent(new MouseEvent('mouseleave', Object.assign({{}}, opts, {{ bubbles: false }})));
                            }}
                        }}
                        if (typeof PointerEvent === 'function') {{
                            el.dispatchEvent(new PointerEvent('pointerover', opts));
                            el.dispatchEvent(new PointerEvent('pointerenter', Object.assign({{}}, opts, {{ bubbles: false }})));
                        }}
                        if (typeof MouseEvent === 'function') {{
                            el.dispatchEvent(new MouseEvent('mouseover', opts));
                            el.dispatchEvent(new MouseEvent('mouseenter', Object.assign({{}}, opts, {{ bubbles: false }})));
                        }}
                        window.__lastHoveredElement = el;
                    }}
                }}

                if (typeof PointerEvent === 'function') {{
                    const pe = new PointerEvent(evType, opts);
                    el.dispatchEvent(pe);
                }}
                if (typeof MouseEvent === 'function') {{
                    const me = new MouseEvent("{mev}", opts);
                    el.dispatchEvent(me);
                }}
                return true;
            }})();
            "#,
            sel = target_selector,
            ev = event_type,
            mev = match event_type {
                "pointerdown" => "mousedown",
                "pointerup" => "mouseup",
                "pointermove" => "mousemove",
                other => other,
            },
            x = client_x,
            y = client_y,
            btn = button,
            btns = if event_type == "pointerdown" || button > 0 { 1 } else { 0 },
        );
        self.execute(&js).unwrap_or(false)
    }

    /// Dispatch a full W3C KeyboardEvent (keydown, keyup, keypress) to activeElement or document
    pub fn dispatch_keyboard_event(
        &mut self,
        event_type: &str,
        key: &str,
        code: &str,
        key_code: u32,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
        repeat: bool,
    ) -> bool {
        let js = format!(
            r#"
            (function() {{
                const target = document.activeElement || document.body || document;
                if (target) {{
                    const opts = {{
                        key: "{}",
                        code: "{}",
                        keyCode: {},
                        which: {},
                        ctrlKey: {},
                        altKey: {},
                        shiftKey: {},
                        metaKey: {},
                        repeat: {},
                        bubbles: true,
                        cancelable: true
                    }};
                    const ev = new KeyboardEvent("{}", opts);
                    target.dispatchEvent(ev);
                    return !ev.defaultPrevented;
                }}
                return false;
            }})();
            "#,
            key.replace('\\', "\\\\").replace('"', "\\\""),
            code.replace('\\', "\\\\").replace('"', "\\\""),
            key_code,
            key_code,
            ctrl,
            alt,
            shift,
            meta,
            repeat,
            event_type
        );
        self.execute(&js).unwrap_or(false)
    }

    /// Synchronize computed layout box geometry (x, y, width, height, scrollWidth, scrollHeight) to V8 DOM registry
    pub fn sync_layout_geometry(&mut self, geom_map: &HashMap<String, (f32, f32, f32, f32, f32, f32)>) {
        if geom_map.is_empty() {
            return;
        }
        let mut json = String::from("{");
        for (i, (key, (x, y, w, h, sw, sh))) in geom_map.iter().enumerate() {
            if i > 0 { json.push(','); }
            json.push_str(&format!(r#""{}":[{},{},{},{},{},{}]"#, key.replace('"', "\\\""), x, y, w, h, sw, sh));
        }
        json.push('}');
        let js = format!("window.__layoutGeometryRegistry = {};", json);
        let _ = self.execute(&js);
    }

    /// Dispatch DOMContentLoaded event when DOM parsing finishes and defer scripts have run
    pub fn dispatch_dom_content_loaded(&mut self) -> bool {
        let js = r#"
            (function() {
                document.readyState = 'interactive';
                const ev = new Event('DOMContentLoaded', { bubbles: true, cancelable: false });
                document.dispatchEvent(ev);
                if (typeof document.onreadystatechange === 'function') {
                    try { document.onreadystatechange(ev); } catch(e) { console.error(e); }
                }
            })();
        "#;
        self.execute(js).unwrap_or(false)
    }

    /// Dispatch load event when page and all resources have loaded
    pub fn dispatch_load_event(&mut self) -> bool {
        let js = r#"
            (function() {
                document.readyState = 'complete';
                const ev = new Event('load', { bubbles: false, cancelable: false });
                window.dispatchEvent(ev);
                document.dispatchEvent(ev);
                if (typeof window.onload === 'function') {
                    try { window.onload(ev); } catch(e) { console.error(e); }
                }
                if (typeof document.onreadystatechange === 'function') {
                    try { document.onreadystatechange(ev); } catch(e) { console.error(e); }
                }
            })();
        "#;
        self.execute(js).unwrap_or(false)
    }

    /// Drain any HTML string written via document.write() during script execution
    pub fn take_written_html(&mut self) -> String {
        CURRENT_CONTEXT.with(|ctx| {
            if let Some(ref mut c) = *ctx.borrow_mut() {
                std::mem::take(&mut c.written_html_buffer)
            } else {
                String::new()
            }
        })
    }

    /// Set parser stream active state to prevent duplicate DOM mutations during initial parse
    pub fn set_parsing(&mut self, parsing: bool) {
        CURRENT_CONTEXT.with(|ctx| {
            if let Some(ref mut c) = *ctx.borrow_mut() {
                c.is_parsing = parsing;
            }
        });
    }
}

// Recursively register all DOM nodes into the registry map
fn register_dom_tree(
    node: &NodePtr,
    registry: &mut HashMap<usize, NodePtr>,
    next_id: &mut usize,
) -> usize {
    let target_ptr = Rc::as_ptr(node);
    let id = if let Some((&existing_id, _)) = registry.iter().find(|(_, n)| Rc::as_ptr(n) == target_ptr) {
        existing_id
    } else {
        let new_id = *next_id;
        *next_id += 1;
        registry.insert(new_id, Rc::clone(node));
        new_id
    };

    let children = node.borrow().children.clone();
    for child in &children {
        register_dom_tree(child, registry, next_id);
    }
    id
}

/// Prune detached nodes from node_registry to prevent memory leaks
fn prune_detached_nodes(ctx: &mut ActiveContext) {
    if let Some(ref root) = ctx.dom_root {
        let mut reachable = HashSet::new();
        collect_reachable_nodes(root, &mut reachable);
        ctx.node_registry.retain(|_, node| reachable.contains(&Rc::as_ptr(node)));
    }
}

fn collect_reachable_nodes(node: &NodePtr, set: &mut HashSet<*const std::cell::RefCell<NodeData>>) {
    set.insert(Rc::as_ptr(node));
    let children = node.borrow().children.clone();
    for child in &children {
        collect_reachable_nodes(child, set);
    }
}

// ============================================================================
// GLOBAL SCOPE BOOTSTRAP (window, console, location, navigator, document, fetch)
// ============================================================================

fn setup_window_global<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
) {
    let window_key = v8::String::new(scope, "window").unwrap();
    global.set(scope, window_key.into(), global.into());

    let global_this_key = v8::String::new(scope, "globalThis").unwrap();
    global.set(scope, global_this_key.into(), global.into());
}

fn setup_console_api<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
) {
    let console_key = v8::String::new(scope, "console").unwrap();
    let console_obj = v8::Object::new(scope);

    let log_key = v8::String::new(scope, "log").unwrap();
    let log_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         _rv: v8::ReturnValue| {
            let mut log_line = String::new();
            for i in 0..args.length() {
                let val_str = args.get(i).to_rust_string_lossy(scope);
                if i > 0 {
                    log_line.push(' ');
                }
                log_line.push_str(&val_str);
            }
            println!("[Axomai V8 Console.log]: {}", log_line);
            CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref mut c) = *ctx.borrow_mut() {
                    c.console_logs.push(log_line);
                }
            });
        },
    )
    .unwrap();
    console_obj.set(scope, log_key.into(), log_fn.into());

    let warn_key = v8::String::new(scope, "warn").unwrap();
    console_obj.set(scope, warn_key.into(), log_fn.into());

    let error_key = v8::String::new(scope, "error").unwrap();
    console_obj.set(scope, error_key.into(), log_fn.into());

    global.set(scope, console_key.into(), console_obj.into());
}

fn setup_location_api<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
    url_str: &str,
) {
    let loc_key = v8::String::new(scope, "location").unwrap();
    let loc_obj = v8::Object::new(scope);

    let href_k = v8::String::new(scope, "href").unwrap();
    let href_v = v8::String::new(scope, url_str).unwrap();
    loc_obj.set(scope, href_k.into(), href_v.into());

    let proto = if url_str.starts_with("https://") {
        "https:"
    } else {
        "http:"
    };
    let proto_k = v8::String::new(scope, "protocol").unwrap();
    let proto_v = v8::String::new(scope, proto).unwrap();
    loc_obj.set(scope, proto_k.into(), proto_v.into());

    global.set(scope, loc_key.into(), loc_obj.into());
}

fn setup_navigator_api<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
) {
    let nav_key = v8::String::new(scope, "navigator").unwrap();
    let nav_obj = v8::Object::new(scope);

    let ua_k = v8::String::new(scope, "userAgent").unwrap();
    let ua_v = v8::String::new(
        scope,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AxomaiBrowser/0.2.0 (Rust/V8)",
    )
    .unwrap();
    nav_obj.set(scope, ua_k.into(), ua_v.into());

    let lang_k = v8::String::new(scope, "language").unwrap();
    let lang_v = v8::String::new(scope, "en-US").unwrap();
    nav_obj.set(scope, lang_k.into(), lang_v.into());

    global.set(scope, nav_key.into(), nav_obj.into());
}

fn setup_timer_apis<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
) {
    let timeout_key = v8::String::new(scope, "setTimeout").unwrap();
    let timeout_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            if args.length() > 0 && args.get(0).is_function() {
                let func_local: v8::Local<v8::Function> = args.get(0).try_into().unwrap();
                let func_global = v8::Global::new(scope, func_local);

                let delay_ms = if args.length() > 1 {
                    args.get(1)
                        .to_integer(scope)
                        .map(|i| i.value().max(0) as u64)
                        .unwrap_or(0)
                } else {
                    0
                };

                let timer_id = NEXT_TIMER_ID.with(|id| {
                    let mut curr = id.borrow_mut();
                    let val = *curr;
                    *curr += 1;
                    val
                });

                PENDING_TIMERS.with(|q| {
                    q.borrow_mut().push(TimerTask {
                        id: timer_id,
                        delay: Duration::from_millis(delay_ms),
                        created_at: Instant::now(),
                        is_interval: false,
                        callback: func_global,
                    });
                });

                rv.set(v8::Integer::new(scope, timer_id as i32).into());
            }
        },
    )
    .unwrap();
    global.set(scope, timeout_key.into(), timeout_fn.into());

    let interval_key = v8::String::new(scope, "setInterval").unwrap();
    let interval_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            if args.length() > 0 && args.get(0).is_function() {
                let func_local: v8::Local<v8::Function> = args.get(0).try_into().unwrap();
                let func_global = v8::Global::new(scope, func_local);

                let delay_ms = if args.length() > 1 {
                    args.get(1)
                        .to_integer(scope)
                        .map(|i| i.value().max(1) as u64)
                        .unwrap_or(1)
                } else {
                    1
                };

                let timer_id = NEXT_TIMER_ID.with(|id| {
                    let mut curr = id.borrow_mut();
                    let val = *curr;
                    *curr += 1;
                    val
                });

                PENDING_TIMERS.with(|q| {
                    q.borrow_mut().push(TimerTask {
                        id: timer_id,
                        delay: Duration::from_millis(delay_ms),
                        created_at: Instant::now(),
                        is_interval: true,
                        callback: func_global,
                    });
                });

                rv.set(v8::Integer::new(scope, timer_id as i32).into());
            }
        },
    )
    .unwrap();
    global.set(scope, interval_key.into(), interval_fn.into());

    let clear_timeout_k = v8::String::new(scope, "clearTimeout").unwrap();
    let clear_timeout_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         _rv: v8::ReturnValue| {
            if args.length() > 0 {
                if let Some(id_int) = args.get(0).to_integer(scope) {
                    let tid = id_int.value() as u32;
                    CANCELLED_TIMERS.with(|c| c.borrow_mut().push(tid));
                }
            }
        },
    )
    .unwrap();
    global.set(scope, clear_timeout_k.into(), clear_timeout_fn.into());

    let clear_interval_k = v8::String::new(scope, "clearInterval").unwrap();
    global.set(scope, clear_interval_k.into(), clear_timeout_fn.into());
}

// ============================================================================
// TRUE ASYNCHRONOUS FETCH() API (Uses Background Worker Thread & Channel)
// ============================================================================

fn setup_async_fetch_api<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
    base_url_str: &str,
    engine_id: usize,
) {
    let base_url = base_url_str.to_string();
    let fetch_key = v8::String::new(scope, "fetch").unwrap();

    let fetch_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              args: v8::FunctionCallbackArguments,
              mut rv: v8::ReturnValue| {
            if args.length() == 0 {
                return;
            }
            let target_url_raw = args.get(0).to_rust_string_lossy(scope);

            // Resolve target URL relative to base URL
            let resolved_url = if let Ok(base) = URL::parse(&base_url) {
                base.resolve(&target_url_raw)
            } else {
                target_url_raw
            };

            // Parse optional init parameter: { method, headers, body }
            let mut method = "GET".to_string();
            let mut custom_headers: Vec<(String, String)> = Vec::new();
            let mut request_body: Option<String> = None;

            if args.length() > 1 && args.get(1).is_object() {
                if let Some(init_obj) = args.get(1).to_object(scope) {
                    // 1. method
                    let method_k = v8::String::new(scope, "method").unwrap();
                    if let Some(val) = init_obj.get(scope, method_k.into()) {
                        if !val.is_undefined() && !val.is_null() {
                            method = val.to_rust_string_lossy(scope);
                        }
                    }

                    // 2. body
                    let body_k = v8::String::new(scope, "body").unwrap();
                    if let Some(val) = init_obj.get(scope, body_k.into()) {
                        if !val.is_undefined() && !val.is_null() {
                            request_body = Some(val.to_rust_string_lossy(scope));
                        }
                    }

                    // 3. headers
                    let headers_k = v8::String::new(scope, "headers").unwrap();
                    if let Some(h_val) = init_obj.get(scope, headers_k.into()) {
                        if h_val.is_object() {
                            if let Some(h_obj) = h_val.to_object(scope) {
                                if let Some(prop_names) = h_obj.get_property_names(scope) {
                                    for i in 0..prop_names.length() {
                                        if let Some(key_val) = prop_names.get_index(scope, i) {
                                            let key_str = key_val.to_rust_string_lossy(scope);
                                            if let Some(v_val) = h_obj.get(scope, key_val) {
                                                let v_str = v_val.to_rust_string_lossy(scope);
                                                custom_headers.push((key_str, v_str));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Allocate unique Request ID for this fetch
            let request_id = NEXT_FETCH_ID.fetch_add(1, Ordering::SeqCst);

            // Create Promise & Resolver on V8 Thread
            let resolver = v8::PromiseResolver::new(scope).unwrap();
            let promise = resolver.get_promise(scope);
            rv.set(promise.into());

            // Store PromiseResolver strictly on V8 thread in thread-local map
            let resolver_global = v8::Global::new(scope, resolver);
            PENDING_FETCH_RESOLVERS.with(|map| {
                map.borrow_mut().insert(request_id, resolver_global);
            });

            // Attach cookies matching origin
            let cookies_to_send = COOKIE_JAR.with(|jar| {
                let map = jar.borrow();
                let orig = get_origin_from_url(&resolved_url);
                if let Some(list) = map.get(&orig) {
                    list.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("; ")
                } else {
                    String::new()
                }
            });
            if !cookies_to_send.is_empty() {
                custom_headers.push(("Cookie".to_string(), cookies_to_send));
            }

            // Spawn background thread to perform non-blocking HTTP request
            // ONLY plain Rust data is moved to worker thread!
            let url_for_worker = resolved_url.clone();
            thread::spawn(move || {
                let mut req = match method.to_uppercase().as_str() {
                    "POST" => ureq::post(&url_for_worker),
                    "PUT" => ureq::put(&url_for_worker),
                    "DELETE" => ureq::delete(&url_for_worker),
                    "PATCH" => ureq::patch(&url_for_worker),
                    "HEAD" => ureq::head(&url_for_worker),
                    _ => ureq::get(&url_for_worker),
                };

                for (k, v) in &custom_headers {
                    req = req.set(k, v);
                }

                let call_res = if let Some(ref body_str) = request_body {
                    req.send_string(body_str)
                } else {
                    req.call()
                };

                match call_res {
                    Ok(resp) => {
                        let status = resp.status();
                        let status_text = resp.status_text().to_string();
                        let mut resp_headers = Vec::new();
                        for header_name in resp.headers_names() {
                            if let Some(val) = resp.header(&header_name) {
                                resp_headers.push((header_name, val.to_string()));
                            }
                        }
                        let body = resp.into_string().unwrap_or_default();
                        push_fetch_result(FetchResult {
                            engine_id,
                            request_id,
                            status,
                            status_text,
                            headers: resp_headers,
                            url: url_for_worker,
                            body,
                            error: None,
                        });
                    }
                    Err(err) => {
                        push_fetch_result(FetchResult {
                            engine_id,
                            request_id,
                            status: 500,
                            status_text: "Internal Error".to_string(),
                            headers: Vec::new(),
                            url: url_for_worker,
                            body: String::new(),
                            error: Some(format!("Fetch Error: {}", err)),
                        });
                    }
                }
            });
        },
    )
    .unwrap();

    global.set(scope, fetch_key.into(), fetch_fn.into());
}

fn get_origin_from_url(url_str: &str) -> String {
    if let Ok(url) = URL::parse(url_str) {
        if url.scheme == "http" || url.scheme == "https" {
            format!("{}://{}:{}", url.scheme, url.host, url.port)
        } else if url.scheme == "file" {
            "file://".to_string()
        } else {
            "null".to_string()
        }
    } else {
        "null".to_string()
    }
}

fn get_storage_dir() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("axomai_browser");
    dir.push("local_storage");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn save_local_storage_to_disk(origin: &str, map: &HashMap<String, String>) {
    let safe_origin = origin.replace([':', '/', '\\', '?', '*', '<', '>', '|', '"'], "_");
    let mut path = get_storage_dir();
    path.push(format!("{}.txt", safe_origin));
    let mut content = String::new();
    for (k, v) in map {
        let ek = k.replace('\\', "\\\\").replace('\n', "\\n").replace('=', "\\=");
        let ev = v.replace('\\', "\\\\").replace('\n', "\\n");
        content.push_str(&format!("{}={}\n", ek, ev));
    }
    let _ = std::fs::write(path, content);
}

fn load_local_storage_from_disk(origin: &str) -> HashMap<String, String> {
    let safe_origin = origin.replace([':', '/', '\\', '?', '*', '<', '>', '|', '"'], "_");
    let mut path = get_storage_dir();
    path.push(format!("{}.txt", safe_origin));
    let mut map = HashMap::new();
    if let Ok(data) = std::fs::read_to_string(path) {
        for line in data.lines() {
            if let Some(idx) = line.find('=') {
                let k = line[..idx].replace("\\=", "=").replace("\\n", "\n").replace("\\\\", "\\");
                let v = line[idx + 1..].replace("\\n", "\n").replace("\\\\", "\\");
                map.insert(k, v);
            }
        }
    }
    map
}

// ============================================================================
// WEB STORAGE (localStorage / sessionStorage) & COOKIE SYSTEM
// ============================================================================

fn setup_storage_and_cookies_api<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
    url_str: &str,
) {
    let origin = get_origin_from_url(url_str);
    let origin_local = origin.clone();

    // 1. window.localStorage & window.sessionStorage bindings
    let create_storage_obj = |s: &mut v8::ContextScope<'s, v8::HandleScope>, is_session: bool| -> v8::Local<'s, v8::Object> {
        let storage = v8::Object::new(s);
        let orig = origin.clone();

        // getItem(key)
        let get_item_k = v8::String::new(s, "getItem").unwrap();
        let orig_c = orig.clone();
        let get_item_fn = v8::Function::new(
            s,
            move |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
                if args.length() == 0 { return; }
                let key = args.get(0).to_rust_string_lossy(scope);
                let val_opt = if is_session {
                    SESSION_STORAGE.with(|st| st.borrow().get(&orig_c).and_then(|m| m.get(&key).cloned()))
                } else {
                    LOCAL_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        if !map.contains_key(&orig_c) {
                            let disk_map = load_local_storage_from_disk(&orig_c);
                            map.insert(orig_c.clone(), disk_map);
                        }
                        map.get(&orig_c).and_then(|m| m.get(&key).cloned())
                    })
                };
                if let Some(val) = val_opt {
                    let v_str = v8::String::new(scope, &val).unwrap();
                    rv.set(v_str.into());
                } else {
                    rv.set(v8::null(scope).into());
                }
            },
        ).unwrap();
        storage.set(s, get_item_k.into(), get_item_fn.into());

        // setItem(key, val)
        let set_item_k = v8::String::new(s, "setItem").unwrap();
        let orig_c = orig.clone();
        let set_item_fn = v8::Function::new(
            s,
            move |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
                if args.length() < 2 { return; }
                let key = args.get(0).to_rust_string_lossy(scope);
                let val = args.get(1).to_rust_string_lossy(scope);
                if is_session {
                    SESSION_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        map.entry(orig_c.clone()).or_insert_with(HashMap::new).insert(key, val);
                    });
                } else {
                    LOCAL_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        if !map.contains_key(&orig_c) {
                            let disk_map = load_local_storage_from_disk(&orig_c);
                            map.insert(orig_c.clone(), disk_map);
                        }
                        if let Some(m) = map.get_mut(&orig_c) {
                            m.insert(key, val);
                            save_local_storage_to_disk(&orig_c, m);
                        }
                    });
                }
            },
        ).unwrap();
        storage.set(s, set_item_k.into(), set_item_fn.into());

        // removeItem(key)
        let rm_item_k = v8::String::new(s, "removeItem").unwrap();
        let orig_c = orig.clone();
        let rm_item_fn = v8::Function::new(
            s,
            move |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
                if args.length() == 0 { return; }
                let key = args.get(0).to_rust_string_lossy(scope);
                if is_session {
                    SESSION_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        if let Some(m) = map.get_mut(&orig_c) {
                            m.remove(&key);
                        }
                    });
                } else {
                    LOCAL_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        if !map.contains_key(&orig_c) {
                            let disk_map = load_local_storage_from_disk(&orig_c);
                            map.insert(orig_c.clone(), disk_map);
                        }
                        if let Some(m) = map.get_mut(&orig_c) {
                            m.remove(&key);
                            save_local_storage_to_disk(&orig_c, m);
                        }
                    });
                }
            },
        ).unwrap();
        storage.set(s, rm_item_k.into(), rm_item_fn.into());

        // clear()
        let clear_k = v8::String::new(s, "clear").unwrap();
        let orig_c = orig.clone();
        let clear_fn = v8::Function::new(
            s,
            move |_scope: &mut v8::HandleScope, _args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
                if is_session {
                    SESSION_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        if let Some(m) = map.get_mut(&orig_c) {
                            m.clear();
                        }
                    });
                } else {
                    LOCAL_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        map.insert(orig_c.clone(), HashMap::new());
                        save_local_storage_to_disk(&orig_c, &HashMap::new());
                    });
                }
            },
        ).unwrap();
        storage.set(s, clear_k.into(), clear_fn.into());

        // key(index)
        let key_k = v8::String::new(s, "key").unwrap();
        let orig_c = orig.clone();
        let key_fn = v8::Function::new(
            s,
            move |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
                if args.length() == 0 { return; }
                let idx = args.get(0).to_integer(scope).map(|i| i.value() as usize).unwrap_or(0);
                let key_opt = if is_session {
                    SESSION_STORAGE.with(|st| st.borrow().get(&orig_c).and_then(|m| m.keys().nth(idx).cloned()))
                } else {
                    LOCAL_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        if !map.contains_key(&orig_c) {
                            let disk_map = load_local_storage_from_disk(&orig_c);
                            map.insert(orig_c.clone(), disk_map);
                        }
                        map.get(&orig_c).and_then(|m| m.keys().nth(idx).cloned())
                    })
                };
                if let Some(k) = key_opt {
                    let k_str = v8::String::new(scope, &k).unwrap();
                    rv.set(k_str.into());
                } else {
                    rv.set(v8::null(scope).into());
                }
            },
        ).unwrap();
        storage.set(s, key_k.into(), key_fn.into());

        // length getter
        let len_k = v8::String::new(s, "length").unwrap();
        let orig_c = orig.clone();
        let len_fn = v8::Function::new(
            s,
            move |scope: &mut v8::HandleScope, _args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
                let len = if is_session {
                    SESSION_STORAGE.with(|st| st.borrow().get(&orig_c).map(|m| m.len()).unwrap_or(0))
                } else {
                    LOCAL_STORAGE.with(|st| {
                        let mut map = st.borrow_mut();
                        if !map.contains_key(&orig_c) {
                            let disk_map = load_local_storage_from_disk(&orig_c);
                            map.insert(orig_c.clone(), disk_map);
                        }
                        map.get(&orig_c).map(|m| m.len()).unwrap_or(0)
                    })
                };
                rv.set(v8::Integer::new(scope, len as i32).into());
            },
        ).unwrap();
        storage.set(s, len_k.into(), len_fn.into());

        storage
    };

    let local_storage = create_storage_obj(scope, false);
    let session_storage = create_storage_obj(scope, true);

    let ls_k = v8::String::new(scope, "localStorage").unwrap();
    let ss_k = v8::String::new(scope, "sessionStorage").unwrap();
    global.set(scope, ls_k.into(), local_storage.into());
    global.set(scope, ss_k.into(), session_storage.into());

    // 2. Cookie native hooks with attribute awareness
    let get_cookie_fn = v8::Function::new(
        scope,
        move |s: &mut v8::HandleScope, _args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let cookies = COOKIE_JAR.with(|jar| {
                let map = jar.borrow();
                if let Some(list) = map.get(&origin_local) {
                    list.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("; ")
                } else {
                    String::new()
                }
            });
            let v_str = v8::String::new(s, &cookies).unwrap();
            rv.set(v_str.into());
        },
    ).unwrap();

    let orig_set = origin.clone();
    let set_cookie_fn = v8::Function::new(
        scope,
        move |s: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            if args.length() == 0 { return; }
            let cookie_str = args.get(0).to_rust_string_lossy(s);
            let parts: Vec<&str> = cookie_str.split(';').collect();
            if let Some(first) = parts.first() {
                let kv: Vec<&str> = first.splitn(2, '=').collect();
                if kv.len() == 2 {
                    let k = kv[0].trim().to_string();
                    let v = kv[1].trim().to_string();
                    COOKIE_JAR.with(|jar| {
                        let mut map = jar.borrow_mut();
                        let list = map.entry(orig_set.clone()).or_insert_with(Vec::new);
                        list.retain(|(item_k, _)| item_k != &k);
                        // Check if deleted via max-age=0 or expires in past
                        let is_deleted = parts.iter().skip(1).any(|p| {
                            let p_lower = p.trim().to_lowercase();
                            p_lower == "max-age=0" || p_lower.starts_with("expires=thu, 01 jan 1970")
                        });
                        if !is_deleted {
                            list.push((k, v));
                        }
                    });
                }
            }
        },
    ).unwrap();

    let doc_k = v8::String::new(scope, "__native_get_cookie").unwrap();
    let set_doc_k = v8::String::new(scope, "__native_set_cookie").unwrap();
    global.set(scope, doc_k.into(), get_cookie_fn.into());
    global.set(scope, set_doc_k.into(), set_cookie_fn.into());
}

// ============================================================================
// DOCUMENT & ELEMENT DOM BINDINGS
// ============================================================================

fn setup_document_api<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
) {
    let doc_key = v8::String::new(scope, "document").unwrap();
    let doc_obj = v8::Object::new(scope);

    // 1. document.write(html_string)
    let write_key = v8::String::new(scope, "write").unwrap();
    let write_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         _rv: v8::ReturnValue| {
            if args.length() > 0 {
                let snippet = args.get(0).to_rust_string_lossy(scope);
                println!("[Axomai V8 document.write]: {}", snippet);

                CURRENT_CONTEXT.with(|ctx| {
                    if let Some(ref mut c) = *ctx.borrow_mut() {
                        c.written_html_buffer.push_str(&snippet);
                        c.dom_mutated = true;

                        // Only mutate DOM directly if NOT during initial parser stream tokenization
                        if !c.is_parsing {
                            if let Some(ref root) = c.dom_root {
                                let snippet_tree = HTMLParser::new(&snippet).parse();
                                let target = find_body(root).unwrap_or_else(|| Rc::clone(root));

                                let children = snippet_tree.borrow().children.clone();
                                if !children.is_empty() {
                                    for child in children {
                                        register_dom_tree(&child, &mut c.node_registry, &mut c.next_node_id);
                                        NodeData::add_child(&target, &child);
                                    }
                                } else {
                                    register_dom_tree(&snippet_tree, &mut c.node_registry, &mut c.next_node_id);
                                    NodeData::add_child(&target, &snippet_tree);
                                }
                                prune_detached_nodes(c);
                            }
                        }
                    }
                });
            }
        },
    )
    .unwrap();
    doc_obj.set(scope, write_key.into(), write_fn.into());

    // 2. document.getElementById(id)
    let get_id_key = v8::String::new(scope, "getElementById").unwrap();
    let get_id_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            if args.length() == 0 {
                rv.set(v8::null(scope).into());
                return;
            }
            let target_id = args.get(0).to_rust_string_lossy(scope);

            let node_match = CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref c) = *ctx.borrow() {
                    if let Some(ref root) = c.dom_root {
                        return find_element_by_id(root, &target_id);
                    }
                }
                None
            });

            if let Some(node) = node_match {
                let elem_obj = wrap_dom_element(scope, &node);
                rv.set(elem_obj.into());
            } else {
                rv.set(v8::null(scope).into());
            }
        },
    )
    .unwrap();
    doc_obj.set(scope, get_id_key.into(), get_id_fn.into());

    // 3. document.querySelector(selector)
    let qs_key = v8::String::new(scope, "querySelector").unwrap();
    let qs_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            if args.length() == 0 {
                rv.set(v8::null(scope).into());
                return;
            }
            let sel = args.get(0).to_rust_string_lossy(scope);

            let node_match = CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref c) = *ctx.borrow() {
                    if let Some(ref root) = c.dom_root {
                        return query_selector(root, &sel);
                    }
                }
                None
            });

            if let Some(node) = node_match {
                let elem_obj = wrap_dom_element(scope, &node);
                rv.set(elem_obj.into());
            } else {
                rv.set(v8::null(scope).into());
            }
        },
    )
    .unwrap();
    doc_obj.set(scope, qs_key.into(), qs_fn.into());

    // 3b. document.querySelectorAll(selector)
    let qsa_key = v8::String::new(scope, "querySelectorAll").unwrap();
    let qsa_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            if args.length() == 0 {
                let arr = v8::Array::new(scope, 0);
                rv.set(arr.into());
                return;
            }
            let sel = args.get(0).to_rust_string_lossy(scope);

            let matching_nodes = CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref c) = *ctx.borrow() {
                    if let Some(ref root) = c.dom_root {
                        return query_selector_all(root, &sel);
                    }
                }
                Vec::new()
            });

            let arr = v8::Array::new(scope, matching_nodes.len() as i32);
            for (i, node) in matching_nodes.iter().enumerate() {
                let elem_obj = wrap_dom_element(scope, node);
                let idx_val = v8::Integer::new(scope, i as i32);
                arr.set(scope, idx_val.into(), elem_obj.into());
            }
            rv.set(arr.into());
        },
    )
    .unwrap();
    doc_obj.set(scope, qsa_key.into(), qsa_fn.into());

    // 4. document.createElement(tagName)
    let ce_key = v8::String::new(scope, "createElement").unwrap();
    let ce_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            let tag = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                "div".to_string()
            };

            let new_node = NodeData::new_element(&tag, HashMap::new());
            let elem_obj = wrap_dom_element(scope, &new_node);
            rv.set(elem_obj.into());
        },
    )
    .unwrap();
    doc_obj.set(scope, ce_key.into(), ce_fn.into());

    // 5. document.body getter
    let body_node = CURRENT_CONTEXT.with(|ctx| {
        if let Some(ref c) = *ctx.borrow() {
            if let Some(ref root) = c.dom_root {
                return find_body(root);
            }
        }
        None
    });
    if let Some(b) = body_node {
        let body_k = v8::String::new(scope, "body").unwrap();
        let body_v = wrap_dom_element(scope, &b);
        doc_obj.set(scope, body_k.into(), body_v.into());
    }

    global.set(scope, doc_key.into(), doc_obj.into());
}

/// Wrap a Rust NodePtr into an Element Object with standard methods and property hooks
fn wrap_dom_element<'s>(
    scope: &mut v8::HandleScope<'s>,
    node: &NodePtr,
) -> v8::Local<'s, v8::Object> {
    let elem_obj = v8::Object::new(scope);
    let node_id = node.borrow().node_id;

    CURRENT_CONTEXT.with(|ctx| {
        let mut opt = ctx.borrow_mut();
        if let Some(ref mut c) = *opt {
            c.node_registry.insert(node_id, Rc::clone(node));
        }
    });

    let id_num = v8::Integer::new(scope, node_id as i32);
    let id_key = v8::String::new(scope, "__nodeId").unwrap();
    elem_obj.set(scope, id_key.into(), id_num.into());
    let id_key2 = v8::String::new(scope, "__node_id").unwrap();
    elem_obj.set(scope, id_key2.into(), id_num.into());

    // 1. appendChild(childElement)
    let append_key = v8::String::new(scope, "appendChild").unwrap();
    let append_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            if args.length() == 0 || !args.get(0).is_object() {
                return;
            }

            let this_obj = args.this();
            let child_obj: v8::Local<v8::Object> = args.get(0).try_into().unwrap();

            let nid_key = v8::String::new(scope, "__nodeId").unwrap();
            let parent_id = this_obj
                .get(scope, nid_key.into())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(0);
            let child_id = child_obj
                .get(scope, nid_key.into())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(0);

            CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref mut c) = *ctx.borrow_mut() {
                    let parent_node = c.node_registry.get(&parent_id).cloned();
                    let child_node = c.node_registry.get(&child_id).cloned();

                    if let (Some(parent), Some(child)) = (parent_node, child_node) {
                        NodeData::add_child(&parent, &child);
                        c.dom_mutated = true;
                    }
                }
            });

            rv.set(args.get(0));
        },
    )
    .unwrap();
    elem_obj.set(scope, append_key.into(), append_fn.into());

    // 2. removeChild(childElement)
    let remove_child_k = v8::String::new(scope, "removeChild").unwrap();
    let remove_child_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            if args.length() == 0 || !args.get(0).is_object() {
                return;
            }

            let this_obj = args.this();
            let child_obj: v8::Local<v8::Object> = args.get(0).try_into().unwrap();

            let nid_key = v8::String::new(scope, "__nodeId").unwrap();
            let parent_id = this_obj
                .get(scope, nid_key.into())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(0);
            let child_id = child_obj
                .get(scope, nid_key.into())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(0);

            CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref mut c) = *ctx.borrow_mut() {
                    let parent_node = c.node_registry.get(&parent_id).cloned();
                    let child_node = c.node_registry.get(&child_id).cloned();

                    if let (Some(parent), Some(child)) = (parent_node, child_node) {
                        NodeData::remove_child(&parent, &child);
                        c.dom_mutated = true;
                        prune_detached_nodes(c);
                    }
                }
            });

            rv.set(args.get(0));
        },
    )
    .unwrap();
    elem_obj.set(scope, remove_child_k.into(), remove_child_fn.into());

    // 3. remove()
    let remove_k = v8::String::new(scope, "remove").unwrap();
    let remove_fn = v8::Function::new(
        scope,
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         _rv: v8::ReturnValue| {
            let this_obj = args.this();
            let nid_key = v8::String::new(scope, "__nodeId").unwrap();
            let node_id = this_obj
                .get(scope, nid_key.into())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(0);

            CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref mut c) = *ctx.borrow_mut() {
                    if let Some(node) = c.node_registry.get(&node_id) {
                        remove_node(node);
                        c.dom_mutated = true;
                        prune_detached_nodes(c);
                    }
                }
            });
        },
    )
    .unwrap();
    elem_obj.set(scope, remove_k.into(), remove_fn.into());

    // 4. Low-level internal getters and setters for innerHTML & textContent
    bind_element_accessor_functions(scope, elem_obj, node_id);

    // 5. Attributes: getAttribute, setAttribute, hasAttribute, removeAttribute
    bind_element_attribute_functions(scope, elem_obj, node_id);

    // 6. Event listeners: addEventListener, removeEventListener, dispatchEvent
    bind_element_event_functions(scope, elem_obj);

    elem_obj
}

// Binds __getInnerHTML, __setInnerHTML, __getTextContent, __setTextContent
fn bind_element_accessor_functions<'s>(
    scope: &mut v8::HandleScope<'s>,
    elem_obj: v8::Local<'s, v8::Object>,
    node_id: usize,
) {
    let get_html_k = v8::String::new(scope, "__getInnerHTML").unwrap();
    let get_html_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              _args: v8::FunctionCallbackArguments,
              mut rv: v8::ReturnValue| {
            let html_str = CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref c) = *ctx.borrow() {
                    if let Some(node) = c.node_registry.get(&node_id) {
                        return get_node_inner_html(node);
                    }
                }
                String::new()
            });
            let v = v8::String::new(scope, &html_str).unwrap();
            rv.set(v.into());
        },
    )
    .unwrap();
    elem_obj.set(scope, get_html_k.into(), get_html_fn.into());

    let set_html_k = v8::String::new(scope, "__setInnerHTML").unwrap();
    let set_html_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              args: v8::FunctionCallbackArguments,
              _rv: v8::ReturnValue| {
            if args.length() > 0 {
                let html_val = args.get(0).to_rust_string_lossy(scope);
                CURRENT_CONTEXT.with(|ctx| {
                    if let Some(ref mut c) = *ctx.borrow_mut() {
                        if let Some(node) = c.node_registry.get(&node_id).cloned() {
                            set_node_inner_html(&node, &html_val);
                            c.dom_mutated = true;
                            register_dom_tree(&node, &mut c.node_registry, &mut c.next_node_id);
                            prune_detached_nodes(c);
                        }
                    }
                });
            }
        },
    )
    .unwrap();
    elem_obj.set(scope, set_html_k.into(), set_html_fn.into());

    let get_text_k = v8::String::new(scope, "__getTextContent").unwrap();
    let get_text_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              _args: v8::FunctionCallbackArguments,
              mut rv: v8::ReturnValue| {
            let text_str = CURRENT_CONTEXT.with(|ctx| {
                if let Some(ref c) = *ctx.borrow() {
                    if let Some(node) = c.node_registry.get(&node_id) {
                        return get_node_text_content(node);
                    }
                }
                String::new()
            });
            let v = v8::String::new(scope, &text_str).unwrap();
            rv.set(v.into());
        },
    )
    .unwrap();
    elem_obj.set(scope, get_text_k.into(), get_text_fn.into());

    let set_text_k = v8::String::new(scope, "__setTextContent").unwrap();
    let set_text_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              args: v8::FunctionCallbackArguments,
              _rv: v8::ReturnValue| {
            if args.length() > 0 {
                let text_val = args.get(0).to_rust_string_lossy(scope);
                CURRENT_CONTEXT.with(|ctx| {
                    if let Some(ref mut c) = *ctx.borrow_mut() {
                        if let Some(node) = c.node_registry.get(&node_id).cloned() {
                            set_node_text_content(&node, &text_val);
                            c.dom_mutated = true;
                        }
                    }
                });
            }
        },
    )
    .unwrap();
    elem_obj.set(scope, set_text_k.into(), set_text_fn.into());
}

fn bind_element_attribute_functions<'s>(
    scope: &mut v8::HandleScope<'s>,
    elem_obj: v8::Local<'s, v8::Object>,
    node_id: usize,
) {
    let get_attr_k = v8::String::new(scope, "getAttribute").unwrap();
    let get_attr_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              args: v8::FunctionCallbackArguments,
              mut rv: v8::ReturnValue| {
            if args.length() > 0 {
                let attr_name = args.get(0).to_rust_string_lossy(scope);
                let val_opt = CURRENT_CONTEXT.with(|ctx| {
                    if let Some(ref c) = *ctx.borrow() {
                        if let Some(node) = c.node_registry.get(&node_id) {
                            let b = node.borrow();
                            if let NodeType::Element { ref attributes, .. } = b.node_type {
                                return attributes.get(&attr_name).cloned();
                            }
                        }
                    }
                    None
                });

                if let Some(val) = val_opt {
                    let v = v8::String::new(scope, &val).unwrap();
                    rv.set(v.into());
                } else {
                    rv.set(v8::null(scope).into());
                }
            }
        },
    )
    .unwrap();
    elem_obj.set(scope, get_attr_k.into(), get_attr_fn.into());

    let set_attr_k = v8::String::new(scope, "setAttribute").unwrap();
    let set_attr_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              args: v8::FunctionCallbackArguments,
              _rv: v8::ReturnValue| {
            if args.length() >= 2 {
                let key = args.get(0).to_rust_string_lossy(scope);
                let val = args.get(1).to_rust_string_lossy(scope);
                CURRENT_CONTEXT.with(|ctx| {
                    if let Some(ref mut c) = *ctx.borrow_mut() {
                        if let Some(node) = c.node_registry.get(&node_id) {
                            let mut b = node.borrow_mut();
                            if let NodeType::Element { ref mut attributes, .. } = b.node_type {
                                attributes.insert(key, val);
                            }
                            c.dom_mutated = true;
                        }
                    }
                });
            }
        },
    )
    .unwrap();
    elem_obj.set(scope, set_attr_k.into(), set_attr_fn.into());
}

fn bind_element_event_functions<'s>(
    scope: &mut v8::HandleScope<'s>,
    elem_obj: v8::Local<'s, v8::Object>,
) {
    let events_map = v8::Object::new(scope);
    let events_k = v8::String::new(scope, "__events").unwrap();
    elem_obj.set(scope, events_k.into(), events_map.into());
}

/// Inject JavaScript bootstrap code that configures:
/// 1. window.addEventListener, document.addEventListener, element.addEventListener
/// 2. Event bubbling & capture phase from target -> parent -> document -> window
/// 3. element.innerHTML, textContent, className, id reactive property getters & setters
fn inject_dom_prototype_bootstrap<'s>(scope: &mut v8::ContextScope<'s, v8::HandleScope>) {
    let bootstrap_js = r#"
    (function() {
        // --------------------------------------------------------------------
        // Web Standards: Headers, Request, Response
        // --------------------------------------------------------------------
        function Headers(init) {
            this._headers = {};
            if (init) {
                if (Array.isArray(init)) {
                    for (let i = 0; i < init.length; i++) {
                        this.set(init[i][0], init[i][1]);
                    }
                } else if (init instanceof Headers) {
                    init.forEach((v, k) => this.set(k, v));
                } else if (typeof init === 'object') {
                    for (const k in init) {
                        this.set(k, init[k]);
                    }
                }
            }
        }
        Headers.prototype.get = function(name) {
            return this._headers[String(name).toLowerCase()] || null;
        };
        Headers.prototype.set = function(name, value) {
            this._headers[String(name).toLowerCase()] = String(value);
        };
        Headers.prototype.has = function(name) {
            return String(name).toLowerCase() in this._headers;
        };
        Headers.prototype.delete = function(name) {
            delete this._headers[String(name).toLowerCase()];
        };
        Headers.prototype.forEach = function(cb, thisArg) {
            for (const k in this._headers) {
                cb.call(thisArg, this._headers[k], k, this);
            }
        };
        window.Headers = Headers;

        window.Request = function(input, init) {
            this.url = typeof input === 'string' ? input : (input ? input.url : '');
            init = init || {};
            this.method = (init.method || 'GET').toUpperCase();
            this.headers = new Headers(init.headers);
            this.body = init.body || null;
        };

        window.Response = function(body, init) {
            init = init || {};
            this.status = init.status || 200;
            this.statusText = init.statusText || 'OK';
            this.ok = this.status >= 200 && this.status < 300;
            this.headers = new Headers(init.headers);
            this._body = String(body || '');
        };
        Response.prototype.text = function() { return Promise.resolve(this._body); };
        Response.prototype.json = function() {
            try {
                return Promise.resolve(JSON.parse(this._body));
            } catch(e) {
                return Promise.reject(e);
            }
        };

        // --------------------------------------------------------------------
        // Web Standards: Event Constructors & Phases
        // --------------------------------------------------------------------
        function Event(type, options) {
            options = options || {};
            this.type = String(type);
            this.bubbles = !!options.bubbles;
            this.cancelable = !!options.cancelable;
            this.defaultPrevented = false;
            this.eventPhase = 0; // NONE
            this.timeStamp = Date.now();
            this._stopped = false;
            this._immediateStopped = false;
            this.target = null;
            this.currentTarget = null;
        }
        Event.NONE = 0;
        Event.CAPTURING_PHASE = 1;
        Event.AT_TARGET = 2;
        Event.BUBBLING_PHASE = 3;

        Event.prototype.preventDefault = function() {
            if (this.cancelable) this.defaultPrevented = true;
        };
        Event.prototype.stopPropagation = function() {
            this._stopped = true;
        };
        Event.prototype.stopImmediatePropagation = function() {
            this._stopped = true;
            this._immediateStopped = true;
        };
        window.Event = Event;

        window.CustomEvent = function(type, options) {
            Event.call(this, type, options);
            this.detail = (options && options.detail !== undefined) ? options.detail : null;
        };
        CustomEvent.prototype = Object.create(Event.prototype);

        window.MouseEvent = function(type, options) {
            Event.call(this, type, options);
            options = options || {};
            this.clientX = options.clientX || 0;
            this.clientY = options.clientY || 0;
            this.button = options.button || 0;
            this.buttons = options.buttons || 0;
        };
        MouseEvent.prototype = Object.create(Event.prototype);

        window.PointerEvent = function(type, options) {
            MouseEvent.call(this, type, options);
            options = options || {};
            this.pointerId = options.pointerId || 1;
            this.width = options.width || 1;
            this.height = options.height || 1;
            this.pressure = options.pressure || (options.buttons ? 0.5 : 0);
            this.tiltX = options.tiltX || 0;
            this.tiltY = options.tiltY || 0;
            this.pointerType = options.pointerType || 'mouse';
            this.isPrimary = options.isPrimary !== undefined ? options.isPrimary : true;
        };
        PointerEvent.prototype = Object.create(MouseEvent.prototype);

        window.KeyboardEvent = function(type, options) {
            Event.call(this, type, options);
            options = options || {};
            this.key = options.key || '';
            this.code = options.code || '';
            this.keyCode = options.keyCode || (this.key.length === 1 ? this.key.toUpperCase().charCodeAt(0) : 0);
            this.which = options.which || this.keyCode;
            this.charCode = options.charCode || (type === 'keypress' ? this.keyCode : 0);
            this.ctrlKey = !!options.ctrlKey;
            this.shiftKey = !!options.shiftKey;
            this.altKey = !!options.altKey;
            this.metaKey = !!options.metaKey;
            this.repeat = !!options.repeat;
            this.location = options.location || 0;
        };
        KeyboardEvent.prototype = Object.create(Event.prototype);

        // Window Scrolling APIs
        window.scrollX = 0;
        window.scrollY = 0;
        window.pageXOffset = 0;
        window.pageYOffset = 0;
        window.scrollTo = function(x, y) {
            let targetX = 0, targetY = 0;
            if (typeof x === 'object' && x !== null) {
                targetX = x.left !== undefined ? x.left : (window.scrollX || 0);
                targetY = x.top !== undefined ? x.top : (window.scrollY || 0);
            } else {
                targetX = Number(x) || 0;
                targetY = Number(y) || 0;
            }
            window.scrollX = targetX;
            window.scrollY = targetY;
            window.pageXOffset = targetX;
            window.pageYOffset = targetY;
        };
        window.scroll = window.scrollTo;
        window.scrollBy = function(dx, dy) {
            let targetX = 0, targetY = 0;
            if (typeof dx === 'object' && dx !== null) {
                targetX = dx.left || 0;
                targetY = dx.top || 0;
            } else {
                targetX = Number(dx) || 0;
                targetY = Number(dy) || 0;
            }
            window.scrollTo(window.scrollX + targetX, window.scrollY + targetY);
        };

        // --------------------------------------------------------------------
        // Universal W3C EventTarget with True Capture & Bubble Phases
        // --------------------------------------------------------------------
        function setupEventTarget(obj) {
            if (!obj) return obj;
            obj.__listeners = {};
            obj.__pointerCaptures = new Set();
            obj.__scrollTop = 0;
            obj.__scrollLeft = 0;
            obj.__scrollHeight = 0;
            obj.__scrollWidth = 0;

            // Pointer Capture Spec APIs
            obj.setPointerCapture = function(pointerId) {
                const pid = Number(pointerId) || 1;
                this.__pointerCaptures.add(pid);
                window.__capturedPointerElements = window.__capturedPointerElements || new Map();
                window.__capturedPointerElements.set(pid, this);
            };

            obj.releasePointerCapture = function(pointerId) {
                const pid = Number(pointerId) || 1;
                this.__pointerCaptures.delete(pid);
                if (window.__capturedPointerElements) {
                    window.__capturedPointerElements.delete(pid);
                }
            };

            obj.hasPointerCapture = function(pointerId) {
                return this.__pointerCaptures.has(Number(pointerId) || 1);
            };

            // Focus Spec APIs
            obj.focus = function() {
                const prevActive = document.activeElement;
                if (prevActive === this) return;
                if (prevActive && typeof prevActive.dispatchEvent === 'function') {
                    prevActive.dispatchEvent(new Event('blur', { bubbles: false }));
                    prevActive.dispatchEvent(new Event('focusout', { bubbles: true }));
                }
                document.activeElement = this;
                this.dispatchEvent(new Event('focus', { bubbles: false }));
                this.dispatchEvent(new Event('focusin', { bubbles: true }));
            };

            obj.blur = function() {
                if (document.activeElement === this) {
                    document.activeElement = document.body;
                    this.dispatchEvent(new Event('blur', { bubbles: false }));
                    this.dispatchEvent(new Event('focusout', { bubbles: true }));
                }
            };

            function getElementGeometry(elem) {
                if (!window.__layoutGeometryRegistry || !elem) return null;
                const nid = (elem.__node_id !== undefined && elem.__node_id !== null) ? elem.__node_id : elem.__nodeId;
                if (nid !== undefined && nid !== null && window.__layoutGeometryRegistry['__node_' + nid]) {
                    return window.__layoutGeometryRegistry['__node_' + nid];
                }
                if (elem.id && window.__layoutGeometryRegistry[elem.id]) {
                    return window.__layoutGeometryRegistry[elem.id];
                }
                if (elem.id && window.__layoutGeometryRegistry['#' + elem.id]) {
                    return window.__layoutGeometryRegistry['#' + elem.id];
                }
                if (elem.className && typeof elem.className === 'string') {
                    const firstCls = elem.className.trim().split(/\s+/)[0];
                    if (firstCls && window.__layoutGeometryRegistry['.' + firstCls]) {
                        return window.__layoutGeometryRegistry['.' + firstCls];
                    }
                }
                if (elem.tagName && window.__layoutGeometryRegistry[elem.tagName.toLowerCase()]) {
                    return window.__layoutGeometryRegistry[elem.tagName.toLowerCase()];
                }
                return null;
            }

            // DOM Geometry Spec APIs
            obj.getBoundingClientRect = function() {
                const geom = getElementGeometry(this);
                const x = geom ? geom[0] : (this.offsetLeft || 0);
                const y = geom ? geom[1] : (this.offsetTop || 0);
                const w = geom ? geom[2] : (this.clientWidth || 0);
                const h = geom ? geom[3] : (this.clientHeight || 0);
                const sx = window.scrollX || 0;
                const sy = window.scrollY || 0;
                const top = y - sy;
                const left = x - sx;
                return {
                    x: left,
                    y: top,
                    width: w,
                    height: h,
                    top: top,
                    right: left + w,
                    bottom: top + h,
                    left: left,
                    toJSON: function() { return this; }
                };
            };

            Object.defineProperty(obj, 'offsetLeft', {
                get: function() {
                    const geom = getElementGeometry(this);
                    if (geom) return geom[0];
                    return this.__offsetLeft || 0;
                },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'offsetTop', {
                get: function() {
                    const geom = getElementGeometry(this);
                    if (geom) return geom[1];
                    return this.__offsetTop || 0;
                },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'offsetWidth', {
                get: function() {
                    const geom = getElementGeometry(this);
                    if (geom) return geom[2];
                    return this.__offsetWidth || 0;
                },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'offsetHeight', {
                get: function() {
                    const geom = getElementGeometry(this);
                    if (geom) return geom[3];
                    return this.__offsetHeight || 0;
                },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'clientWidth', {
                get: function() { return this.offsetWidth; },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'clientHeight', {
                get: function() { return this.offsetHeight; },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'clientTop', {
                get: function() { return 0; },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'clientLeft', {
                get: function() { return 0; },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'offsetParent', {
                get: function() {
                    let p = this.parentElement;
                    while (p) {
                        if (p.style && (p.style.position === 'relative' || p.style.position === 'absolute' || p.style.position === 'fixed')) {
                            return p;
                        }
                        p = p.parentElement;
                    }
                    return document.body;
                },
                configurable: true,
                enumerable: true
            });

            // Element Scrolling APIs
            Object.defineProperty(obj, 'scrollTop', {
                get: function() { return this.__scrollTop || 0; },
                set: function(val) {
                    this.__scrollTop = Math.max(0, Number(val) || 0);
                    const targetId = (this.__node_id !== undefined && this.__node_id !== null) ? ('__node_' + this.__node_id) : (this.id || '');
                    if (typeof window.__native_set_element_scroll === 'function' && targetId) {
                        window.__native_set_element_scroll(targetId, this.__scrollLeft || 0, this.__scrollTop);
                    }
                },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'scrollLeft', {
                get: function() { return this.__scrollLeft || 0; },
                set: function(val) {
                    this.__scrollLeft = Math.max(0, Number(val) || 0);
                    const targetId = (this.__node_id !== undefined && this.__node_id !== null) ? ('__node_' + this.__node_id) : (this.id || '');
                    if (typeof window.__native_set_element_scroll === 'function' && targetId) {
                        window.__native_set_element_scroll(targetId, this.__scrollLeft, this.__scrollTop || 0);
                    }
                },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'scrollHeight', {
                get: function() {
                    const geom = getElementGeometry(this);
                    if (geom && geom[5] > 0) return geom[5];
                    return this.__scrollHeight || this.offsetHeight;
                },
                set: function(val) { this.__scrollHeight = Number(val) || 0; },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(obj, 'scrollWidth', {
                get: function() {
                    const geom = getElementGeometry(this);
                    if (geom && geom[4] > 0) return geom[4];
                    return this.__scrollWidth || this.offsetWidth;
                },
                set: function(val) { this.__scrollWidth = Number(val) || 0; },
                configurable: true,
                enumerable: true
            });

            obj.scrollTo = function(x, y) {
                let targetX = 0, targetY = 0;
                if (typeof x === 'object' && x !== null) {
                    targetX = x.left !== undefined ? x.left : this.scrollLeft;
                    targetY = x.top !== undefined ? x.top : this.scrollTop;
                } else {
                    targetX = Number(x) || 0;
                    targetY = Number(y) || 0;
                }
                this.scrollLeft = targetX;
                this.scrollTop = targetY;
            };
            obj.scroll = obj.scrollTo;

            obj.scrollBy = function(dx, dy) {
                let targetX = 0, targetY = 0;
                if (typeof dx === 'object' && dx !== null) {
                    targetX = dx.left || 0;
                    targetY = dx.top || 0;
                } else {
                    targetX = Number(dx) || 0;
                    targetY = Number(dy) || 0;
                }
                this.scrollTo(this.scrollLeft + targetX, this.scrollTop + targetY);
            };

            obj.addEventListener = function(type, listener, options) {
                if (!listener) return;
                if (!this.__listeners[type]) this.__listeners[type] = [];

                let capture = false;
                let once = false;
                let passive = false;
                let signal = null;

                if (typeof options === 'boolean') {
                    capture = options;
                } else if (typeof options === 'object' && options !== null) {
                    capture = !!options.capture;
                    once = !!options.once;
                    passive = !!options.passive;
                    signal = options.signal;
                }

                if (signal) {
                    if (signal.aborted) return;
                    signal.addEventListener('abort', () => {
                        this.removeEventListener(type, listener, { capture });
                    });
                }

                const exists = this.__listeners[type].some(l => l.listener === listener && l.capture === capture);
                if (!exists) {
                    this.__listeners[type].push({ listener, capture, once, passive });
                }
            };

            obj.removeEventListener = function(type, listener, options) {
                if (!this.__listeners || !this.__listeners[type]) return;
                let capture = false;
                if (typeof options === 'boolean') {
                    capture = options;
                } else if (typeof options === 'object' && options !== null) {
                    capture = !!options.capture;
                }
                this.__listeners[type] = this.__listeners[type].filter(l => !(l.listener === listener && l.capture === capture));
            };

            obj.dispatchEvent = function(event) {
                if (typeof event === 'string') {
                    event = new Event(event, { bubbles: true, cancelable: true });
                }
                if (!event.target) event.target = this;

                // Build full hierarchy propagation path: [target, parent, ..., document, window]
                const path = [];
                let curr = this;
                while (curr) {
                    path.push(curr);
                    curr = curr.parentElement;
                }
                if (typeof document !== 'undefined' && path.indexOf(document) === -1) {
                    path.push(document);
                }
                if (typeof window !== 'undefined' && path.indexOf(window) === -1) {
                    path.push(window);
                }

                // 1. CAPTURING PHASE (Window down to target's parent)
                event.eventPhase = 1; // Event.CAPTURING_PHASE
                for (let i = path.length - 1; i > 0; i--) {
                    if (event._immediateStopped || event._stopped) break;
                    const node = path[i];
                    event.currentTarget = node;
                    if (node.__listeners && node.__listeners[event.type]) {
                        const list = node.__listeners[event.type].slice();
                        for (let j = 0; j < list.length; j++) {
                            const l = list[j];
                            if (l.capture) {
                                try { l.listener.call(node, event); } catch(e) { console.error(e); }
                                if (l.once) node.removeEventListener(event.type, l.listener, true);
                                if (event._immediateStopped) break;
                            }
                        }
                    }
                }

                // 2. AT TARGET PHASE
                if (!event._immediateStopped) {
                    event.eventPhase = 2; // Event.AT_TARGET
                    event.currentTarget = this;

                    // Inline on[type] handler
                    if (typeof this['on' + event.type] === 'function') {
                        try { this['on' + event.type].call(this, event); } catch(e) { console.error(e); }
                    }

                    if (!event._immediateStopped && this.__listeners && this.__listeners[event.type]) {
                        const list = this.__listeners[event.type].slice();
                        for (let j = 0; j < list.length; j++) {
                            const l = list[j];
                            try { l.listener.call(this, event); } catch(e) { console.error(e); }
                            if (l.once) this.removeEventListener(event.type, l.listener, l.capture);
                            if (event._immediateStopped) break;
                        }
                    }
                }

                // 3. BUBBLING PHASE (Target's parent up to Window)
                if (event.bubbles && !event._stopped && !event._immediateStopped) {
                    event.eventPhase = 3; // Event.BUBBLING_PHASE
                    for (let i = 1; i < path.length; i++) {
                        if (event._immediateStopped || event._stopped) break;
                        const node = path[i];
                        event.currentTarget = node;

                        if (typeof node['on' + event.type] === 'function') {
                            try { node['on' + event.type].call(node, event); } catch(e) { console.error(e); }
                        }

                        if (!event._immediateStopped && node.__listeners && node.__listeners[event.type]) {
                            const list = node.__listeners[event.type].slice();
                            for (let j = 0; j < list.length; j++) {
                                const l = list[j];
                                if (!l.capture) {
                                    try { l.listener.call(node, event); } catch(e) { console.error(e); }
                                    if (l.once) node.removeEventListener(event.type, l.listener, false);
                                    if (event._immediateStopped) break;
                                }
                            }
                        }
                    }
                }

                event.eventPhase = 0; // Event.NONE
                return !event.defaultPrevented;
            };
            return obj;
        }

        // Setup EventTarget on window and document
        setupEventTarget(window);
        setupEventTarget(document);
        document.readyState = 'loading';

        // Setup document.cookie getter/setter
        Object.defineProperty(document, 'cookie', {
            get: function() {
                return (typeof window.__native_get_cookie === 'function') ? window.__native_get_cookie() : '';
            },
            set: function(val) {
                if (typeof window.__native_set_cookie === 'function') {
                    window.__native_set_cookie(String(val));
                }
            },
            configurable: true,
            enumerable: true
        });

        // W3C queueMicrotask
        window.queueMicrotask = function(callback) {
            if (typeof callback === 'function') {
                Promise.resolve().then(callback).catch(function(e) { console.error(e); });
            }
        };

        // Performance & Compositor Animation Frame Queue
        window.performance = window.performance || {
            now: function() { return Date.now(); }
        };

        window.__rafCallbacks = new Map();
        window.__nextRafId = 1;

        window.requestAnimationFrame = function(callback) {
            if (typeof callback !== 'function') return 0;
            const id = window.__nextRafId++;
            window.__rafCallbacks.set(id, callback);
            return id;
        };

        window.cancelAnimationFrame = function(id) {
            window.__rafCallbacks.delete(id);
        };

        window.__flushAnimationFrameCallbacks = function(now) {
            if (!window.__rafCallbacks || window.__rafCallbacks.size === 0) return false;
            const cbs = Array.from(window.__rafCallbacks.values());
            window.__rafCallbacks.clear();
            for (let i = 0; i < cbs.length; i++) {
                try { cbs[i](now); } catch(e) { console.error(e); }
            }
            return true;
        };

        // Browser ES Module State Machine, Dependency Loader & Live Binding Registry
        window.__moduleRegistry = new Map();
        window.__moduleRecords = new Map();
        window.__resolvingModules = new Map();

        const MODULE_UNLINKED = 0;
        const MODULE_LINKING = 1;
        const MODULE_LINKED = 2;
        const MODULE_EVALUATING = 3;
        const MODULE_EVALUATED = 4;

        window.__resolveModuleUrl = function(specifier, base) {
            try {
                return new URL(specifier, base || window.location.href).href;
            } catch(e) {
                return specifier;
            }
        };

        // AST-safe ES Module Transformer that strips comments, preserves strings, and emits live getters
        window.__transformESModule = function(source) {
            let i = 0;
            const len = source.length;
            let cleanSource = '';
            let inString = false;
            let strQuote = '';

            while (i < len) {
                const ch = source[i];
                const next = (i + 1 < len) ? source[i + 1] : '';

                if (inString) {
                    cleanSource += ch;
                    if (ch === '\\' && i + 1 < len) {
                        cleanSource += source[++i];
                    } else if (ch === strQuote) {
                        inString = false;
                    }
                    i++;
                    continue;
                }

                // Line comment
                if (ch === '/' && next === '/') {
                    while (i < len && source[i] !== '\n') i++;
                    cleanSource += '\n';
                    continue;
                }

                // Block comment
                if (ch === '/' && next === '*') {
                    i += 2;
                    while (i + 1 < len && !(source[i] === '*' && source[i+1] === '/')) i++;
                    i += 2;
                    cleanSource += ' ';
                    continue;
                }

                // String literal
                if (ch === '"' || ch === "'" || ch === '`') {
                    inString = true;
                    strQuote = ch;
                    cleanSource += ch;
                    i++;
                    continue;
                }

                cleanSource += ch;
                i++;
            }

            let importHeaders = '';
            const identMap = Object.create(null);
            let importIndex = 0;

            let transformed = cleanSource;

            // 1. import defaultExport, { a, b as c } from "specifier";
            transformed = transformed.replace(
                /import\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*,\s*\{([^}]+)\}\s+from\s*['"]([^'"]+)['"]\s*;?/g,
                function(m, def, named, spec) {
                    const modVar = '__import_mod_' + (importIndex++);
                    importHeaders += 'const ' + modVar + ' = await importModule("' + spec + '");\n';
                    identMap[def] = modVar + '.default';
                    const items = named.split(',');
                    for (let p = 0; p < items.length; p++) {
                        const item = items[p].trim();
                        if (!item) continue;
                        if (item.indexOf(' as ') !== -1) {
                            const pair = item.split(' as ');
                            identMap[pair[1].trim()] = modVar + '["' + pair[0].trim() + '"]';
                        } else {
                            identMap[item] = modVar + '["' + item + '"]';
                        }
                    }
                    return '';
                }
            );

            // 2. import defaultExport from "specifier";
            transformed = transformed.replace(
                /import\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s+from\s*['"]([^'"]+)['"]\s*;?/g,
                function(m, def, spec) {
                    const modVar = '__import_mod_' + (importIndex++);
                    importHeaders += 'const ' + modVar + ' = await importModule("' + spec + '");\n';
                    identMap[def] = modVar + '.default';
                    return '';
                }
            );

            // 3. import * as name from "specifier";
            transformed = transformed.replace(
                /import\s*\*\s*as\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s+from\s*['"]([^'"]+)['"]\s*;?/g,
                function(m, name, spec) {
                    importHeaders += 'const ' + name + ' = await importModule("' + spec + '");\n';
                    return '';
                }
            );

            // 4. import { a, b as c } from "specifier";
            transformed = transformed.replace(
                /import\s*\{([^}]+)\}\s*from\s*['"]([^'"]+)['"]\s*;?/g,
                function(m, bindings, specifier) {
                    const modVar = '__import_mod_' + (importIndex++);
                    importHeaders += 'const ' + modVar + ' = await importModule("' + specifier + '");\n';
                    const items = bindings.split(',');
                    for (let p = 0; p < items.length; p++) {
                        const item = items[p].trim();
                        if (!item) continue;
                        if (item.indexOf(' as ') !== -1) {
                            const pair = item.split(' as ');
                            identMap[pair[1].trim()] = modVar + '["' + pair[0].trim() + '"]';
                        } else {
                            identMap[item] = modVar + '["' + item + '"]';
                        }
                    }
                    return '';
                }
            );

            // 5. import "specifier";
            transformed = transformed.replace(
                /import\s*['"]([^'"]+)['"]\s*;?/g,
                function(m, spec) {
                    importHeaders += 'await importModule("' + spec + '");\n';
                    return '';
                }
            );

            // 6. export default expression;
            transformed = transformed.replace(
                /export\s+default\s+([^;]+);?/g,
                'const __defaultExport = $1; Object.defineProperty(exports, "default", { get: () => __defaultExport, enumerable: true, configurable: true });'
            );

            // 7. export const/let/var decls; -> with LIVE GETTERS
            transformed = transformed.replace(
                /export\s+(const|let|var)\s+([^;]+);?/g,
                function(match, declType, decls) {
                    let res = declType + ' ' + decls + ';\n';
                    const parts = decls.split(',');
                    for (let p = 0; p < parts.length; p++) {
                        const name = parts[p].split('=')[0].trim();
                        if (name && /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(name)) {
                            res += 'Object.defineProperty(exports, "' + name + '", { get: () => ' + name + ', enumerable: true, configurable: true });\n';
                        }
                    }
                    return res;
                }
            );

            // 8. export function name(...) { ... }
            transformed = transformed.replace(
                /export\s+function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)/g,
                'Object.defineProperty(exports, "$1", { get: () => $1, enumerable: true, configurable: true }); function $1'
            );

            // 9. export async function name(...) { ... }
            transformed = transformed.replace(
                /export\s+async\s+function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)/g,
                'Object.defineProperty(exports, "$1", { get: () => $1, enumerable: true, configurable: true }); async function $1'
            );

            // 10. export class name { ... }
            transformed = transformed.replace(
                /export\s+class\s+([a-zA-Z_$][a-zA-Z0-9_$]*)/g,
                'Object.defineProperty(exports, "$1", { get: () => $1, enumerable: true, configurable: true }); class $1'
            );

            // 11. export { a, b as c }; -> LIVE GETTERS
            transformed = transformed.replace(
                /export\s*\{([^}]+)\}\s*;?/g,
                function(match, bindings) {
                    const items = bindings.split(',');
                    let res = '';
                    for (let p = 0; p < items.length; p++) {
                        const item = items[p].trim();
                        if (!item) continue;
                        if (item.indexOf(' as ') !== -1) {
                            const pair = item.split(' as ');
                            const srcName = pair[0].trim();
                            const exportName = pair[1].trim();
                            res += 'Object.defineProperty(exports, "' + exportName + '", { get: () => ' + srcName + ', enumerable: true, configurable: true });\n';
                        } else {
                            res += 'Object.defineProperty(exports, "' + item + '", { get: () => ' + item + ', enumerable: true, configurable: true });\n';
                        }
                    }
                    return res;
                }
            );

            // Scope-aware token scanner to rewrite imported identifiers to live getters
            const keys = Object.keys(identMap);
            if (keys.length > 0) {
                let rewritten = '';
                let idx = 0;
                const tLen = transformed.length;
                let prevNonSpaceChar = '';
                let lastDeclKeyword = '';
                let inParamParen = 0;
                let currentParamScope = null;

                // Stack of Sets of shadowed local variable/param names
                const scopeStack = [new Set()];

                while (idx < tLen) {
                    const ch = transformed[idx];

                    // String literal scanning (preserve quotes and escapes)
                    if (ch === '"' || ch === "'" || ch === '`') {
                        const quote = ch;
                        rewritten += ch;
                        idx++;
                        while (idx < tLen) {
                            const sc = transformed[idx];
                            rewritten += sc;
                            if (sc === '\\' && idx + 1 < tLen) {
                                rewritten += transformed[++idx];
                            } else if (sc === quote) {
                                idx++;
                                break;
                            }
                            idx++;
                        }
                        prevNonSpaceChar = quote;
                        continue;
                    }

                    // Scope blocks { and }
                    if (ch === '{') {
                        const newScope = currentParamScope || new Set();
                        currentParamScope = null;
                        scopeStack.push(newScope);
                        rewritten += ch;
                        prevNonSpaceChar = '{';
                        idx++;
                        continue;
                    }
                    if (ch === '}') {
                        if (scopeStack.length > 1) {
                            scopeStack.pop();
                        }
                        rewritten += ch;
                        prevNonSpaceChar = '}';
                        idx++;
                        continue;
                    }

                    // Parameter list tracking: (a, b, c) => or function(a, b, c)
                    if (ch === '(') {
                        if (prevNonSpaceChar === 'function' || prevNonSpaceChar === '>' || lastDeclKeyword === 'function') {
                            inParamParen++;
                            if (!currentParamScope) currentParamScope = new Set();
                        }
                        rewritten += ch;
                        prevNonSpaceChar = '(';
                        idx++;
                        continue;
                    }
                    if (ch === ')') {
                        if (inParamParen > 0) inParamParen--;
                        rewritten += ch;
                        prevNonSpaceChar = ')';
                        idx++;
                        continue;
                    }

                    // Identifier scanning
                    if (/[a-zA-Z_$]/.test(ch)) {
                        let idStart = idx;
                        while (idx < tLen && /[a-zA-Z0-9_$]/.test(transformed[idx])) {
                            idx++;
                        }
                        const idText = transformed.slice(idStart, idx);

                        // Check declaration keywords
                        if (idText === 'var' || idText === 'let' || idText === 'const' || idText === 'function' || idText === 'class') {
                            lastDeclKeyword = idText;
                            rewritten += idText;
                            prevNonSpaceChar = idText;
                            continue;
                        }

                        // Local variable or function declaration
                        if (lastDeclKeyword) {
                            const curScope = scopeStack[scopeStack.length - 1];
                            curScope.add(idText);
                            lastDeclKeyword = '';
                            rewritten += idText;
                            prevNonSpaceChar = idText[idText.length - 1];
                            continue;
                        }

                        // Function parameter
                        if (inParamParen > 0 && currentParamScope) {
                            currentParamScope.add(idText);
                            rewritten += idText;
                            prevNonSpaceChar = idText[idText.length - 1];
                            continue;
                        }

                        // Check if identifier is shadowed in any active lexical scope
                        let isShadowed = false;
                        for (let s = scopeStack.length - 1; s >= 0; s--) {
                            if (scopeStack[s].has(idText)) {
                                isShadowed = true;
                                break;
                            }
                        }

                        if (!isShadowed && prevNonSpaceChar !== '.' && identMap[idText]) {
                            let peek = idx;
                            while (peek < tLen && /\s/.test(transformed[peek])) peek++;

                            // Object literal explicit key: `{ count: 123 }` or `{ count: val }`
                            if (peek < tLen && transformed[peek] === ':' && (prevNonSpaceChar === '{' || prevNonSpaceChar === ',')) {
                                rewritten += idText;
                            } else if (prevNonSpaceChar === '{' || prevNonSpaceChar === ',') {
                                // Object literal shorthand: `{ count }` or `{ a, count }`
                                if (peek < tLen && (transformed[peek] === '}' || transformed[peek] === ',')) {
                                    rewritten += idText + ': ' + identMap[idText];
                                } else {
                                    rewritten += identMap[idText];
                                }
                            } else {
                                rewritten += identMap[idText];
                            }
                        } else {
                            rewritten += idText;
                        }
                        prevNonSpaceChar = idText[idText.length - 1];
                        continue;
                    }

                    if (!/\s/.test(ch)) {
                        prevNonSpaceChar = ch;
                    }
                    rewritten += ch;
                    idx++;
                }
                transformed = rewritten;
            }

            return importHeaders + transformed;
        };

        window.__executeModule = async function(source, moduleUrl) {
            const resolvedUrl = window.__resolveModuleUrl(moduleUrl || 'inline-module', window.location.href);

            let record = window.__moduleRecords.get(resolvedUrl);
            if (record && record.status === MODULE_EVALUATED) {
                return record.namespace;
            }

            if (!record) {
                record = {
                    url: resolvedUrl,
                    status: MODULE_LINKING,
                    namespace: Object.create(null),
                    source: source
                };
                window.__moduleRecords.set(resolvedUrl, record);
                window.__moduleRegistry.set(resolvedUrl, record.namespace);
            }

            const transformed = window.__transformESModule(source);
            record.status = MODULE_EVALUATING;

            const AsyncFunction = Object.getPrototypeOf(async function(){}).constructor;
            const moduleFn = new AsyncFunction('exports', 'importModule', 'moduleUrl',
                "'use strict';\n" + transformed + "\n//# sourceURL=" + resolvedUrl
            );

            try {
                await moduleFn(record.namespace, window.import, resolvedUrl);
                record.status = MODULE_EVALUATED;
            } catch(e) {
                console.error('[Axomai ESModule Exception in ' + resolvedUrl + ']:', e);
            }

            return record.namespace;
        };

        window.import = function(specifier) {
            const resolvedUrl = window.__resolveModuleUrl(specifier, window.location.href);
            if (window.__moduleRecords.has(resolvedUrl)) {
                const rec = window.__moduleRecords.get(resolvedUrl);
                return Promise.resolve(rec.namespace);
            }
            if (window.__resolvingModules.has(resolvedUrl)) {
                return window.__resolvingModules.get(resolvedUrl);
            }

            const loadPromise = fetch(resolvedUrl)
                .then(function(res) {
                    if (!res.ok) throw new Error('Failed to fetch module: ' + resolvedUrl + ' (status ' + res.status + ')');
                    return res.text();
                })
                .then(function(source) {
                    return window.__executeModule(source, resolvedUrl);
                })
                .finally(function() {
                    window.__resolvingModules.delete(resolvedUrl);
                });

            window.__resolvingModules.set(resolvedUrl, loadPromise);
            return loadPromise;
        };

        // Standard getters/setters definition on Element objects
        window.__setupElementProperties = function(el) {
            if (!el || el.__protoHooked) return el;
            el.__protoHooked = true;

            setupEventTarget(el);

            Object.defineProperty(el, 'innerHTML', {
                get: function() { return this.__getInnerHTML ? this.__getInnerHTML() : ''; },
                set: function(val) { if (this.__setInnerHTML) this.__setInnerHTML(String(val)); },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(el, 'textContent', {
                get: function() { return this.__getTextContent ? this.__getTextContent() : ''; },
                set: function(val) { if (this.__setTextContent) this.__setTextContent(String(val)); },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(el, 'className', {
                get: function() { return this.getAttribute ? (this.getAttribute('class') || '') : ''; },
                set: function(val) { if (this.setAttribute) this.setAttribute('class', String(val)); },
                configurable: true,
                enumerable: true
            });

            Object.defineProperty(el, 'id', {
                get: function() { return this.getAttribute ? (this.getAttribute('id') || '') : ''; },
                set: function(val) { if (this.setAttribute) this.setAttribute('id', String(val)); },
                configurable: true,
                enumerable: true
            });

            return el;
        };

        // Wrap document.createElement, getElementById, querySelector to auto-hook properties
        const origCreate = document.createElement;
        document.createElement = function(tag) {
            const el = origCreate.call(document, tag);
            return window.__setupElementProperties(el);
        };

        const origGetId = document.getElementById;
        document.getElementById = function(id) {
            const el = origGetId.call(document, id);
            return el ? window.__setupElementProperties(el) : null;
        };

        const origQuery = document.querySelector;
        document.querySelector = function(sel) {
            const el = origQuery.call(document, sel);
            return el ? window.__setupElementProperties(el) : null;
        };

        const origQueryAll = document.querySelectorAll;
        if (origQueryAll) {
            document.querySelectorAll = function(sel) {
                const list = origQueryAll.call(document, sel);
                if (Array.isArray(list)) {
                    return list.map(function(el) { return window.__setupElementProperties(el); });
                }
                return list;
            };
        }

        if (document.body) {
            window.__setupElementProperties(document.body);
        }
    })();
    "#;

    if let Some(code) = v8::String::new(scope, bootstrap_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }
}
