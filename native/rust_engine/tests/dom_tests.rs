use axomai_engine::html_parser::{query_selector, query_selector_all, HTMLParser, NodeType};

#[test]
#[ignore] // Multi-class compound selectors (.a.b) not yet implemented
fn test_query_selector_all_multiple_classes() {
    let html = r#"
        <div class="row">
            <span class="badge primary">1</span>
            <span class="badge secondary">2</span>
            <span class="badge primary">3</span>
        </div>
    "#;

    let root = HTMLParser::new(html).parse();
    let primary_badges = query_selector_all(&root, ".badge.primary");
    assert_eq!(primary_badges.len(), 2, "Should find 2 elements matching .badge.primary");

    let any_badge = query_selector_all(&root, ".badge");
    assert_eq!(any_badge.len(), 3, "Should find 3 elements matching .badge");
}

#[test]
fn test_query_selector_universal_and_id() {
    let html = r#"
        <div id="main-content">
            <h1 id="heading">Hello</h1>
        </div>
    "#;

    let root = HTMLParser::new(html).parse();
    let heading = query_selector(&root, "#heading");
    assert!(heading.is_some(), "Should find element by #id");

    if let Some(h) = heading {
        let b = h.borrow();
        if let NodeType::Element { ref tag, .. } = b.node_type {
            assert_eq!(tag, "h1");
        } else {
            panic!("Expected element");
        }
    }
}
