use crate::html_parser::{
    find_body, find_element_by_id, get_node_inner_html, get_node_text_content, query_selector,
    remove_node, set_node_inner_html, set_node_text_content, HTMLParser, NodeData, NodePtr,
    NodeType,
};
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
}

// Timer Task for the Browser Event Loop
struct TimerTask {
    id: u32,
    delay: Duration,
    created_at: Instant,
    is_interval: bool,
    callback: v8::Global<v8::Function>,
}

pub struct V8JSEngine {
    isolate: Option<v8::OwnedIsolate>,
    timers: Vec<TimerTask>,
    next_timer_id: u32,
}

impl V8JSEngine {
    pub fn new() -> Self {
        ensure_v8_initialized();

        let isolate = v8::Isolate::new(Default::default());

        Self {
            isolate: Some(isolate),
            timers: Vec::new(),
            next_timer_id: 1,
        }
    }

    /// Execute a JavaScript snippet against a given DOM tree and URL
    pub fn execute(
        &mut self,
        source: &str,
        dom_root: Option<&NodePtr>,
        url_str: &str,
    ) -> Result<bool, String> {
        let isolate = self.isolate.as_mut().ok_or("V8 Isolate not available")?;

        // 1. Initialize Thread-Local Context with Node Registry
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

        // 2. Set up V8 HandleScope & ContextScope
        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(handle_scope, Default::default());
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let global = context.global(scope);

        // 3. Setup Browser Globals & Web APIs
        setup_window_global(scope, global);
        setup_console_api(scope, global);
        setup_location_api(scope, global, url_str);
        setup_navigator_api(scope, global);
        setup_document_api(scope, global);
        setup_timer_apis(scope, global);

        // 4. Inject DOM Element Prototype Helpers (getters/setters for innerHTML, textContent, appendChild, etc.)
        inject_dom_prototype_bootstrap(scope);

        // 5. Compile & Run JavaScript Script
        let code = v8::String::new(scope, source).ok_or("Failed to allocate JS source string")?;

        let script = match v8::Script::compile(scope, code, None) {
            Some(s) => s,
            None => {
                CURRENT_CONTEXT.with(|ctx| *ctx.borrow_mut() = None);
                return Err("V8 Compilation failed".to_string());
            }
        };

        let _ = script.run(scope);

        // 6. Check if DOM was mutated during script run
        let was_mutated = CURRENT_CONTEXT.with(|ctx| {
            let mut opt = ctx.borrow_mut();
            if let Some(c) = opt.take() {
                c.dom_mutated
            } else {
                false
            }
        });

        Ok(was_mutated)
    }

    /// Process queued timers and microtasks (Event Loop tick)
    pub fn process_event_loop(&mut self) -> bool {
        let isolate = match self.isolate.as_mut() {
            Some(iso) => iso,
            None => return false,
        };

        let now = Instant::now();
        let mut executed_any = false;

        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(handle_scope, Default::default());
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let mut remaining_timers = Vec::new();

        for timer in self.timers.drain(..) {
            if now.duration_since(timer.created_at) >= timer.delay {
                let func = timer.callback.open(scope);
                let recv = v8::undefined(scope).into();
                let _ = func.call(scope, recv, &[]);
                executed_any = true;

                if timer.is_interval {
                    remaining_timers.push(TimerTask {
                        id: timer.id,
                        delay: timer.delay,
                        created_at: Instant::now(),
                        is_interval: true,
                        callback: timer.callback,
                    });
                }
            } else {
                remaining_timers.push(timer);
            }
        }

        self.timers = remaining_timers;
        executed_any
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
// GLOBAL SCOPE BOOTSTRAP (window, console, location, navigator, document)
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
                let func: v8::Local<v8::Function> = args.get(0).try_into().unwrap();
                let recv = v8::undefined(scope).into();
                let _ = func.call(scope, recv, &[]);
            }
            rv.set(v8::Integer::new(scope, 1).into());
        },
    )
    .unwrap();
    global.set(scope, timeout_key.into(), timeout_fn.into());

    let interval_key = v8::String::new(scope, "setInterval").unwrap();
    global.set(scope, interval_key.into(), timeout_fn.into());
}

// ============================================================================
// DOCUMENT & ELEMENT DOM BINDINGS (appendChild, removeChild, innerHTML, textContent)
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
                            "[Axomai V8 DOM] appendChild succeeded for Node #{} into Node #{}",
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
    // __getInnerHTML
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

    // __setInnerHTML
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
                            // Re-index children into registry
                            register_dom_tree(&node, &mut c.node_registry, &mut c.next_node_id);
                        }
                    }
                });
            }
        },
    )
    .unwrap();
    elem_obj.set(scope, set_html_k.into(), set_html_fn.into());

    // __getTextContent
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

    // __setTextContent
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

// Binds getAttribute, setAttribute, hasAttribute
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

// Binds addEventListener, removeEventListener, dispatchEvent
fn bind_element_event_functions<'s>(
    scope: &mut v8::HandleScope<'s>,
    elem_obj: v8::Local<'s, v8::Object>,
) {
    let events_map = v8::Object::new(scope);
    let events_k = v8::String::new(scope, "__events").unwrap();
    elem_obj.set(scope, events_k.into(), events_map.into());
}

/// Inject JavaScript bootstrap code that configures standard W3C getters & setters:
/// element.innerHTML = "..."
/// element.textContent = "..."
/// element.className = "..."
/// element.id = "..."
fn inject_dom_prototype_bootstrap<'s>(scope: &mut v8::ContextScope<'s, v8::HandleScope>) {
    let bootstrap_js = r#"
    (function() {
        const proto = Object.prototype;
        // Standard getters/setters definition on Element objects
        window.__setupElementProperties = function(el) {
            if (!el || el.__protoHooked) return el;
            el.__protoHooked = true;

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

            // Event target methods
            el.addEventListener = function(type, listener) {
                if (!this.__events) this.__events = {};
                if (!this.__events[type]) this.__events[type] = [];
                this.__events[type].push(listener);
            };

            el.removeEventListener = function(type, listener) {
                if (!this.__events || !this.__events[type]) return;
                this.__events[type] = this.__events[type].filter(l => l !== listener);
            };

            el.dispatchEvent = function(event) {
                const type = (typeof event === 'string') ? event : (event && event.type ? event.type : 'click');
                if (typeof this['on' + type] === 'function') {
                    this['on' + type].call(this, event);
                }
                if (this.__events && this.__events[type]) {
                    this.__events[type].forEach(l => l.call(this, event));
                }
            };

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
