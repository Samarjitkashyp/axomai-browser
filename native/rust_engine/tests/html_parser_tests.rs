use axomai_engine::html_parser::{HTMLParser, NodeType};

#[test]
fn test_html_entity_decoding() {
    let html = "<p>&lt;Hello &amp; World&gt; &#39;Test&#39; &quot;Quotes&quot;</p>";
    let root = HTMLParser::new(html).parse();
    let root_borrow = root.borrow();
    let body = root_borrow.children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "body"
        } else {
            false
        }
    }).expect("Body should exist");

    let body_borrow = body.borrow();
    let p = body_borrow.children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "p"
        } else {
            false
        }
    }).expect("Paragraph should exist");

    let p_borrow = p.borrow();
    let text_child = p_borrow.children.first().expect("Text child should exist");
    let tc_borrow = text_child.borrow();
    if let NodeType::Text { ref text } = tc_borrow.node_type {
        assert_eq!(text, "<Hello & World> 'Test' \"Quotes\"");
    } else {
        panic!("Expected text node");
    }
}

#[test]
fn test_implicit_tbody_insertion() {
    let html = "<table><tr><td>Cell 1</td><td>Cell 2</td></tr></table>";
    let root = HTMLParser::new(html).parse();
    let root_borrow = root.borrow();
    let body = root_borrow.children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "body"
        } else {
            false
        }
    }).expect("Body should exist");

    let body_borrow = body.borrow();
    let table = body_borrow.children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "table"
        } else {
            false
        }
    }).expect("Table should exist");

    let table_borrow = table.borrow();
    let tbody = table_borrow.children.first().expect("Table should have tbody child");
    let tbody_borrow = tbody.borrow();
    if let NodeType::Element { ref tag, .. } = tbody_borrow.node_type {
        assert_eq!(tag, "tbody");
    } else {
        panic!("Expected tbody element");
    }
}

#[test]
fn test_self_closing_void_tags() {
    let html = "<div><img src=\"test.png\"><input type=\"text\"><br><p>After</p></div>";
    let root = HTMLParser::new(html).parse();
    let root_borrow = root.borrow();
    let body = root_borrow.children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "body"
        } else {
            false
        }
    }).expect("Body should exist");

    let div = body.borrow().children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "div"
        } else {
            false
        }
    }).expect("Div should exist").clone();

    let div_borrow = div.borrow();
    let tags: Vec<String> = div_borrow.children.iter().filter_map(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            Some(tag.clone())
        } else {
            None
        }
    }).collect();

    assert_eq!(tags, vec!["img", "input", "br", "p"]);
}
