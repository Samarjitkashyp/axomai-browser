use crate::html_parser::{HTMLParser, NodeData, NodePtr, NodeType};
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

pub struct V8JSEngine {
    isolate: Option<v8::OwnedIsolate>,
    pub console_logs: Vec<String>,
    pub dom_mutations: Vec<String>,
}

impl V8JSEngine {
    pub fn new() -> Self {
        ensure_v8_initialized();

        let isolate = v8::Isolate::new(Default::default());

        Self {
            isolate: Some(isolate),
            console_logs: Vec::new(),
            dom_mutations: Vec::new(),
        }
    }

    pub fn execute(&mut self, source: &str, dom_root: Option<&NodePtr>) -> Result<String, String> {
        let isolate = self.isolate.as_mut().ok_or("V8 Isolate not available")?;

        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(handle_scope, Default::default());
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let global = context.global(scope);

        // 1. Setup `console.log`
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
                    let arg = args.get(i);
                    let val_str = arg.to_rust_string_lossy(scope);
                    if i > 0 {
                        log_line.push(' ');
                    }
                    log_line.push_str(&val_str);
                }
                println!("[V8 Console]: {}", log_line);
            },
        )
        .unwrap();

        console_obj.set(scope, log_key.into(), log_fn.into());
        global.set(scope, console_key.into(), console_obj.into());

        // 2. Setup `document` API bindings
        let doc_key = v8::String::new(scope, "document").unwrap();
        let doc_obj = v8::Object::new(scope);

        // document.write callback
        let write_key = v8::String::new(scope, "write").unwrap();
        let write_fn = v8::Function::new(
            scope,
            |scope: &mut v8::HandleScope,
             args: v8::FunctionCallbackArguments,
             _rv: v8::ReturnValue| {
                if args.length() > 0 {
                    let snippet = args.get(0).to_rust_string_lossy(scope);
                    println!("[V8 document.write]: {}", snippet);
                }
            },
        )
        .unwrap();
        doc_obj.set(scope, write_key.into(), write_fn.into());

        global.set(scope, doc_key.into(), doc_obj.into());

        // 3. Compile and Run JavaScript Code via V8
        let code = v8::String::new(scope, source).ok_or("Failed to allocate JS source string")?;

        let script = v8::Script::compile(scope, code, None).ok_or("V8 Compilation failed")?;

        let result = script.run(scope).ok_or("V8 Execution failed")?;

        // 4. Handle DOM mutations if any document.write occurred
        if let Some(root) = dom_root {
            // Apply mutations if detected
            for snippet in &self.dom_mutations {
                let snippet_node = HTMLParser::new(snippet).parse();
                let target = find_body(root).unwrap_or_else(|| Rc::clone(root));

                let snippet_is_html = {
                    let b = snippet_node.borrow();
                    if let NodeType::Element { ref tag, .. } = b.node_type {
                        tag == "html"
                    } else {
                        false
                    }
                };

                if snippet_is_html {
                    let children = snippet_node.borrow().children.clone();
                    for child in children {
                        NodeData::add_child(&target, &child);
                    }
                } else {
                    NodeData::add_child(&target, &snippet_node);
                }
            }
        }

        Ok(result.to_rust_string_lossy(scope))
    }
}

fn find_body(node: &NodePtr) -> Option<NodePtr> {
    let node_borrow = node.borrow();
    if let NodeType::Element { ref tag, .. } = node_borrow.node_type {
        if tag == "body" {
            return Some(Rc::clone(node));
        }
    }
    for child in &node_borrow.children {
        if let Some(body) = find_body(child) {
            return Some(body);
        }
    }
    None
}
