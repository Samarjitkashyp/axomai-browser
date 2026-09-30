use axomai_engine::css_parser::{style_tree, CSSParser, DEFAULT_UA_STYLES};
use axomai_engine::html_parser::{HTMLParser, NodeType};

#[test]
fn test_css_variables_resolution() {
    let html = r#"
        <html>
            <head>
                <style>
                    :root {
                        --main-color: #ff0000;
                        --box-width: 320px;
                    }
                    .card {
                        color: var(--main-color, black);
                        width: var(--box-width, 100px);
                    }
                </style>
            </head>
            <body>
                <div class="card">Hello Variable</div>
            </body>
        </html>
    "#;

    let root = HTMLParser::new(html).parse();
    let author_css = r#"
        :root { --main-color: #ff0000; --box-width: 320px; }
        .card { color: var(--main-color, black); width: var(--box-width, 100px); }
    "#;

    let ua_rules = CSSParser::new(DEFAULT_UA_STYLES).parse();
    let mut all_rules = ua_rules;
    all_rules.extend(CSSParser::new(author_css).parse());

    style_tree(&root, &all_rules);

    let root_borrow = root.borrow();
    let body = root_borrow.children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "body"
        } else {
            false
        }
    }).expect("Body should exist");

    let card = body.borrow().children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "div"
        } else {
            false
        }
    }).expect("Card should exist").clone();

    let card_borrow = card.borrow();
    if let NodeType::Element { ref style, .. } = card_borrow.node_type {
        assert_eq!(style.get("color").map(|s| s.as_str()), Some("#ff0000"));
        assert_eq!(style.get("width").map(|s| s.as_str()), Some("320px"));
    } else {
        panic!("Expected card element");
    }
}

#[test]
fn test_calc_and_clamp_math_functions() {
    let author_css = r#"
        .math-box {
            width: calc(100px + 50px);
            font-size: clamp(12px, 16px, 24px);
        }
    "#;

    let html = "<html><body><div class=\"math-box\">Math Content</div></body></html>";
    let root = HTMLParser::new(html).parse();

    let rules = CSSParser::new(author_css).parse();
    style_tree(&root, &rules);

    let root_borrow = root.borrow();
    let body = root_borrow.children.iter().find(|c| {
        if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
            tag == "body"
        } else {
            false
        }
    }).expect("Body should exist");

    let box_elem = body.borrow().children.first().expect("Child should exist").clone();
    let box_borrow = box_elem.borrow();
    if let NodeType::Element { ref style, .. } = box_borrow.node_type {
        assert_eq!(style.get("width").map(|s| s.as_str()), Some("150.00px"));
        assert_eq!(style.get("font-size").map(|s| s.as_str()), Some("16.00px"));
    } else {
        panic!("Expected element");
    }
}
