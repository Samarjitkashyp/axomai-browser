use axomai_engine::css_parser::{style_tree, CSSParser, DEFAULT_UA_STYLES};
use axomai_engine::html_parser::HTMLParser;
use axomai_engine::layout::build_layout_tree;

#[test]
fn test_table_layout_dimensions() {
    let html = r#"
        <table style="width: 400px;">
            <tr>
                <td>Cell A</td>
                <td>Cell B</td>
            </tr>
            <tr>
                <td>Cell C</td>
                <td>Cell D</td>
            </tr>
        </table>
    "#;

    let root = HTMLParser::new(html).parse();
    let ua_rules = CSSParser::new(DEFAULT_UA_STYLES).parse();
    style_tree(&root, &ua_rules);

    let mut layout = build_layout_tree(&root, None).expect("Layout tree should build");
    let total_h = layout.layout(0.0, 0.0, 800.0);

    assert!(total_h > 0.0, "Table total height should be positive");
}
