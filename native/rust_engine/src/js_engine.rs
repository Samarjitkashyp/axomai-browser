use crate::html_parser::{
    find_body, find_element_by_id, get_node_inner_html, get_node_text_content, query_selector,
    remove_node, set_node_inner_html, set_node_text_content, HTMLParser, NodeData, NodePtr,
    NodeType,
};
use crate::network::URL;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Once;
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
// ACTIVE DOM & RUNTIME EXECUTION CONTEXT
// ============================================================================

struct ActiveContext {
    dom_root: Option<NodePtr>,
    current_url: String,
    dom_mutated: bool,
    console_logs: Vec<String>,
    node_registry: HashMap<usize, NodePtr>,
    next_node_id: usize,
}

thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<ActiveContext>> = RefCell::new(None);
    static PENDING_TIMERS: RefCell<Vec<TimerTask>> = RefCell::new(Vec::new());
    static CANCELLED_TIMERS: RefCell<Vec<u32>> = RefCell::new(Vec::new());
    static NEXT_TIMER_ID: RefCell<u32> = RefCell::new(1);
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
    isolate: Option<v8::OwnedIsolate>,
    page_context: Option<v8::Global<v8::Context>>,
    timers: Vec<TimerTask>,
}

impl V8JSEngine {
    pub fn new() -> Self {
        ensure_v8_initialized();

        let isolate = v8::Isolate::new(Default::default());

        Self {
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

        // 1. Clear old timers and cancelled timer lists
        self.timers.clear();
        PENDING_TIMERS.with(|q| q.borrow_mut().clear());
        CANCELLED_TIMERS.with(|q| q.borrow_mut().clear());

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
        setup_fetch_api(scope, global, url_str);

        // 5. Inject DOM & Window/Document Prototype Event Helpers
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

        // Setup TryCatch block to accurately capture JS exceptions (line, message, stack)
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

        // 1. CRITICAL: Perform Microtask Checkpoint so Promises (Promise.resolve, .then, async/await) resolve!
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

    /// Process queued timers and run Microtask Checkpoint (Real Browser Event Loop tick)
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

        let mut remaining = Vec::new();

        for timer in self.timers.drain(..) {
            // Check if timer was cancelled
            let is_cancelled = CANCELLED_TIMERS.with(|c| c.borrow().contains(&timer.id));
            if is_cancelled {
                continue;
            }

            if now.duration_since(timer.created_at) >= timer.delay {
                // ACTUALLY DELAYED: Executes only after the real elapsed time!
                let func = timer.callback.open(scope);
                let recv = context.global(scope).into();

                let try_catch = &mut v8::TryCatch::new(scope);
                let _ = func.call(try_catch, recv, &[]);

                if let Some(exception) = try_catch.exception() {
                    let msg = exception.to_rust_string_lossy(try_catch);
                    eprintln!("[Axomai V8 Timer Exception]: {}", msg);
                }

                // Run Microtask Checkpoint after each timer callback (Promises inside setTimeout)
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

        // Harvest any newly queued timers from timer callbacks
        PENDING_TIMERS.with(|q| {
            self.timers.append(&mut *q.borrow_mut());
        });

        // Extra Microtask Checkpoint to drain any pending async jobs
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

    /// Dispatch a native click event with full Event object (type, target, bubbles)
    pub fn dispatch_click_event(&mut self, target_selector: &str, click_x: f32, click_y: f32) -> bool {
        let js = format!(
            r#"
            (function() {{
                const el = document.querySelector("{sel}") || document.getElementById("{sel}");
                if (el) {{
                    const event = {{
                        type: 'click',
                        target: el,
                        currentTarget: el,
                        clientX: {x},
                        clientY: {y},
                        bubbles: true,
                        cancelable: true,
                        defaultPrevented: false,
                        preventDefault: function() {{ this.defaultPrevented = true; }},
                        stopPropagation: function() {{ this._stopped = true; }}
                    }};
                    el.dispatchEvent(event);
                    return true;
                }}
                return false;
            }})();
            "#,
            sel = target_selector,
            x = click_x,
            y = click_y
        );
        self.execute(&js).unwrap_or(false)
    }
}

// Recursively register all DOM nodes into the registry map
fn register_dom_tree(
    node: &NodePtr,
    registry: &mut HashMap<usize, NodePtr>,
    next_id: &mut usize,
) -> usize {
    let id = *next_id;
    *next_id += 1;
    registry.insert(id, Rc::clone(node));

    let children = node.borrow().children.clone();
    for child in &children {
        register_dom_tree(child, registry, next_id);
    }
    id
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
    // 1. setTimeout(callback, delay_ms)
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

    // 2. setInterval(callback, delay_ms)
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

    // 3. clearTimeout(timer_id) & clearInterval(timer_id)
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
// NATIVE FETCH() WEB API IMPLEMENTATION (Returns Promise<Response>)
// ============================================================================

fn setup_fetch_api<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
    base_url_str: &str,
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

            let resolver = v8::PromiseResolver::new(scope).unwrap();
            let promise = resolver.get_promise(scope);
            rv.set(promise.into());

            // Perform HTTP GET request via ureq networking layer
            let fetch_result = ureq::get(&resolved_url).call();

            match fetch_result {
                Ok(response) => {
                    let status_code = response.status();
                    let ok_bool = status_code >= 200 && status_code < 300;
                    let body_text = response.into_string().unwrap_or_default();

                    let resp_obj = v8::Object::new(scope);

                    // status & ok
                    let status_k = v8::String::new(scope, "status").unwrap();
                    let status_v = v8::Integer::new(scope, status_code as i32);
                    resp_obj.set(scope, status_k.into(), status_v.into());

                    let ok_k = v8::String::new(scope, "ok").unwrap();
                    let ok_v = v8::Boolean::new(scope, ok_bool);
                    resp_obj.set(scope, ok_k.into(), ok_v.into());

                    // response.text() -> returns Promise resolving to body_text
                    let text_k = v8::String::new(scope, "text").unwrap();
                    let body_clone = body_text.clone();
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

                    // response.json() -> returns Promise resolving to parsed JSON
                    let json_k = v8::String::new(scope, "json").unwrap();
                    let body_json = body_text.clone();
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
                                let err = v8::String::new(s, "Invalid JSON").unwrap();
                                json_res.reject(s, err.into());
                            }
                            r.set(json_p.into());
                        },
                    )
                    .unwrap();
                    resp_obj.set(scope, json_k.into(), json_fn.into());

                    resolver.resolve(scope, resp_obj.into());
                }
                Err(err) => {
                    let err_msg = format!("Network Error: {}", err);
                    let err_v = v8::String::new(scope, &err_msg).unwrap();
                    resolver.reject(scope, err_v.into());
                }
            }
        },
    )
    .unwrap();

    global.set(scope, fetch_key.into(), fetch_fn.into());
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
                            c.dom_mutated = true;
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

    // Find or register node id
    let node_id = CURRENT_CONTEXT.with(|ctx| {
        let mut opt = ctx.borrow_mut();
        if let Some(ref mut c) = *opt {
            for (&id, existing_node) in &c.node_registry {
                if Rc::ptr_eq(existing_node, node) {
                    return id;
                }
            }
            let id = c.next_node_id;
            c.next_node_id += 1;
            c.node_registry.insert(id, Rc::clone(node));
            id
        } else {
            0
        }
    });

    let id_num = v8::Integer::new(scope, node_id as i32);
    let id_key = v8::String::new(scope, "__nodeId").unwrap();
    elem_obj.set(scope, id_key.into(), id_num.into());

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
                        println!(
                            "[Axomai V8 DOM] appendChild: Child Node #{} appended to Parent Node #{}",
                            child_id, parent_id
                        );
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
/// 1. window.addEventListener, document.addEventListener
/// 2. element.innerHTML, textContent, className, id reactive property getters & setters
/// 3. Event bubbling, preventDefault, stopPropagation
fn inject_dom_prototype_bootstrap<'s>(scope: &mut v8::ContextScope<'s, v8::HandleScope>) {
    let bootstrap_js = r#"
    (function() {
        // Universal EventTarget implementation
        function setupEventTarget(obj) {
            if (!obj) return obj;
            obj.__events = {};
            obj.addEventListener = function(type, listener) {
                if (!this.__events[type]) this.__events[type] = [];
                this.__events[type].push(listener);
            };
            obj.removeEventListener = function(type, listener) {
                if (!this.__events || !this.__events[type]) return;
                this.__events[type] = this.__events[type].filter(l => l !== listener);
            };
            obj.dispatchEvent = function(event) {
                const evt = (typeof event === 'string') ? { type: event } : event;
                if (!evt.target) evt.target = this;
                if (typeof this['on' + evt.type] === 'function') {
                    this['on' + evt.type].call(this, evt);
                }
                if (this.__events && this.__events[evt.type]) {
                    this.__events[evt.type].forEach(l => l.call(this, evt));
                }
            };
            return obj;
        }

        // Setup EventTarget on window and document
        setupEventTarget(window);
        setupEventTarget(document);

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
