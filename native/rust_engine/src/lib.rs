pub mod css_parser;
pub mod js_engine;

use css_parser::CSSParser;
use js_engine::RustJSEngine;

// C-FFI interface functions for cross-language bindings

#[no_mangle]
pub extern "C" fn rust_parse_css_count(css_text: *const std::os::raw::c_char) -> usize {
    if css_text.is_null() {
        return 0;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(css_text) };
    if let Ok(str_slice) = c_str.to_str() {
        let parser = CSSParser::new(str_slice);
        return parser.parse().len();
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_css_parser() {
        let css = "h1 { color: red; } p.intro { font-size: 16px; }";
        let parser = CSSParser::new(css);
        let rules = parser.parse();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selector, "h1");
        assert_eq!(rules[0].declarations.get("color").unwrap(), "red");
    }

    #[test]
    fn test_rust_js_engine() {
        let mut js_engine = RustJSEngine::new();
        let js = "var name = 'Axomai'; document.write('<h1>' + name + '</h1>');";
        let mutated = js_engine.execute(js);
        assert!(mutated);
        assert_eq!(js_engine.dom_mutations[0], "<h1>Axomai</h1>");
    }
}
