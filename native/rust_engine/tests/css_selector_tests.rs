use axomai_engine::css_parser::parse_selector;
use axomai_engine::html_parser::{query_selector, query_selector_all, HTMLParser};

#[test]
fn test_complex_combinator_queries() {
    let html = r#"
        <div class="container">
            <h1>Title</h1>
            <p class="lead">First paragraph</p>
            <p>Second paragraph</p>
            <ul>
                <li>Item 1</li>
                <li class="selected">Item 2</li>
            </ul>
        </div>
    "#;
    let root = HTMLParser::new(html).parse();

    // 1. Direct child >
    let q_child = query_selector(&root, "div.container > h1");
    assert!(q_child.is_some(), "Direct child should match");

    // 2. Adjacent sibling +
    let q_adj = query_selector(&root, "h1 + p.lead");
    assert!(q_adj.is_some(), "Adjacent sibling should match");

    // 3. Descendant combinator
    let q_desc = query_selector_all(&root, "div ul li");
    assert_eq!(q_desc.len(), 2, "Descendant query should find 2 li items");

    // 4. Attribute query
    let q_attr = query_selector(&root, "[class=\"selected\"]");
    assert!(q_attr.is_some(), "Attribute exact match should work");
}

#[test]
fn test_pseudo_class_selectors() {
    let html = r#"
        <div class="list">
            <span class="item">First</span>
            <span class="item">Second</span>
            <span class="item">Third</span>
        </div>
        <input type="text" disabled value="Disabled input">
    "#;
    let root = HTMLParser::new(html).parse();

    // :first-child
    let q_first = query_selector(&root, ".item:first-child");
    assert!(q_first.is_some(), ":first-child should match");

    // :last-child
    let q_last = query_selector(&root, ".item:last-child");
    assert!(q_last.is_some(), ":last-child should match");

    // :disabled
    let q_dis = query_selector(&root, "input:disabled");
    assert!(q_dis.is_some(), ":disabled should match");
}
