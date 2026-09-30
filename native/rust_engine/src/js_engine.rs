use crate::html_parser::{
    find_body, find_element_by_id, get_node_text_content, query_selector, set_node_inner_html,
    set_node_text_content, HTMLParser, NodeData, NodePtr, NodeType,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Once;
use v8;

static V8_INIT: Once = Once::new();

fn ensure_v8_initialized() {
    V8_INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

/// Active execution context for the current script execution cycle
struct ActiveContext {
    dom_root: Option<NodePtr>,
    current_url: String,
    dom_mutated: bool,
    console_logs: Vec<String>,
}

thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<ActiveContext>> = RefCell::new(None);
}

pub struct V8JSEngine {
    isolate: Option<v8::OwnedIsolate>,
}

impl V8JSEngine {
    pub fn new() -> Self {
        ensure_v8_initialized();

        let isolate = v8::Isolate::new(Default::default());

        Self {
            isolate: Some(isolate),
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

        // 1. Set the active thread-local context for this script run
        CURRENT_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = Some(ActiveContext {
                dom_root: dom_root.map(Rc::clone),
                current_url: url_str.to_string(),
                dom_mutated: false,
                console_logs: Vec::new(),
            });
        });

        // 2. Set up V8 HandleScope & ContextScope
        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(handle_scope, Default::default());
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let global = context.global(scope);

        // 3. Inject `window` (points to global object)
        let window_key = v8::String::new(scope, "window").unwrap();
        global.set(scope, window_key.into(), global.into());

        // 4. Inject `console` Object (log, warn, error, info)
        setup_console_api(scope, global);

        // 5. Inject `location` Object (href, origin, hostname, protocol, pathname)
        setup_location_api(scope, global, url_str);

        // 6. Inject `navigator` Object (userAgent, language, platform)
        setup_navigator_api(scope, global);

        // 7. Inject `document` Object (getElementById, querySelector, createElement, write, body, title)
        setup_document_api(scope, global);

        // 8. Inject `setTimeout` & `setInterval`
        setup_timer_apis(scope, global);

        // 9. Compile & Run JavaScript
        let code = v8::String::new(scope, source).ok_or("Failed to allocate JS source string")?;

        let script = match v8::Script::compile(scope, code, None) {
            Some(s) => s,
            None => {
                CURRENT_CONTEXT.with(|ctx| *ctx.borrow_mut() = None);
                return Err("V8 Compilation failed".to_string());
            }
        };

        let _result = script.run(scope);

        // 10. Extract whether DOM was mutated during script run
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
}

// ============================================================================
// WEB API BINDINGS
// ============================================================================

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
    // Basic setTimeout implementation executing immediate invocation in browser cycle
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
}

// ============================================================================
// DOM API BINDINGS (document.write, getElementById, querySelector, createElement)
// ============================================================================

fn setup_document_api<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope>,
    global: v8::Local<v8::Object>,
) {
    let doc_key = v8::String::new(scope, "document").unwrap();
    let doc_obj = v8::Object::new(scope);

    // 1. document.write(html_string) -> directly parsed and appended to body in DOM!
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
                                    NodeData::add_child(&target, &child);
                                }
                            } else {
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
                let elem_obj = create_js_element_wrapper(scope, &node);
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
                let elem_obj = create_js_element_wrapper(scope, &node);
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
            let elem_obj = create_js_element_wrapper(scope, &new_node);
            rv.set(elem_obj.into());
        },
    )
    .unwrap();
    doc_obj.set(scope, ce_key.into(), ce_fn.into());

    // 5. document.body property
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
        let body_v = create_js_element_wrapper(scope, &b);
        doc_obj.set(scope, body_k.into(), body_v.into());
    }

    global.set(scope, doc_key.into(), doc_obj.into());
}

/// Create a wrapped JS DOM Element object that binds innerHTML, textContent, style, and attributes
fn create_js_element_wrapper<'s>(
    scope: &mut v8::HandleScope<'s>,
    node: &NodePtr,
) -> v8::Local<'s, v8::Object> {
    let elem_obj = v8::Object::new(scope);

    let (tag, id, current_text) = {
        let b = node.borrow();
        let tag = match &b.node_type {
            NodeType::Element { tag, .. } => tag.to_uppercase(),
            NodeType::Text { .. } => "#text".to_string(),
        };
        let id = match &b.node_type {
            NodeType::Element { attributes, .. } => attributes.get("id").cloned().unwrap_or_default(),
            _ => String::new(),
        };
        let text = get_node_text_content(node);
        (tag, id, text)
    };

    // tagName
    let tag_k = v8::String::new(scope, "tagName").unwrap();
    let tag_v = v8::String::new(scope, &tag).unwrap();
    elem_obj.set(scope, tag_k.into(), tag_v.into());

    // id
    let id_k = v8::String::new(scope, "id").unwrap();
    let id_v = v8::String::new(scope, &id).unwrap();
    elem_obj.set(scope, id_k.into(), id_v.into());

    // textContent
    let tc_k = v8::String::new(scope, "textContent").unwrap();
    let tc_v = v8::String::new(scope, &current_text).unwrap();
    elem_obj.set(scope, tc_k.into(), tc_v.into());

    // innerHTML method/property setInnerHTML
    let set_html_k = v8::String::new(scope, "setInnerHTML").unwrap();
    let node_clone = Rc::clone(node);
    let set_html_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              args: v8::FunctionCallbackArguments,
              _rv: v8::ReturnValue| {
            if args.length() > 0 {
                let html_val = args.get(0).to_rust_string_lossy(scope);
                set_node_inner_html(&node_clone, &html_val);
                CURRENT_CONTEXT.with(|ctx| {
                    if let Some(ref mut c) = *ctx.borrow_mut() {
                        c.dom_mutated = true;
                    }
                });
            }
        },
    )
    .unwrap();
    elem_obj.set(scope, set_html_k.into(), set_html_fn.into());

    // setAttribute(name, value)
    let node_attr_clone = Rc::clone(node);
    let set_attr_k = v8::String::new(scope, "setAttribute").unwrap();
    let set_attr_fn = v8::Function::new(
        scope,
        move |scope: &mut v8::HandleScope,
              args: v8::FunctionCallbackArguments,
              _rv: v8::ReturnValue| {
            if args.length() >= 2 {
                let key = args.get(0).to_rust_string_lossy(scope);
                let val = args.get(1).to_rust_string_lossy(scope);
                let mut b = node_attr_clone.borrow_mut();
                if let NodeType::Element { ref mut attributes, .. } = b.node_type {
                    attributes.insert(key, val);
                }
            }
        },
    )
    .unwrap();
    elem_obj.set(scope, set_attr_k.into(), set_attr_fn.into());

    elem_obj
}
