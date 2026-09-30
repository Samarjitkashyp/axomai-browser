use rust_engine::css_parser::CSSParser;
use rust_engine::html_parser::{find_element_by_id, HTMLParser, NodeType};
use rust_engine::layout::build_layout_tree;

#[test]
fn test_select_and_option_dom_parsing() {
    let html = r#"
        <html>
            <body>
                <form id="contact-form">
                    <select id="country-select" name="country">
                        <option value="in" selected>India</option>
                        <option value="us">United States</option>
                        <option value="uk">United Kingdom</option>
                    </select>
                </form>
            </body>
        </html>
    "#;

    let mut parser = HTMLParser::new();
    let root = parser.parse(html);

    let select_node = find_element_by_id(&root, "country-select");
    assert!(select_node.is_some(), "Select element should be found by ID");

    let sn = select_node.unwrap();
    let sn_borrow = sn.borrow();
    assert_eq!(sn_borrow.children.len(), 3, "Select should have 3 option children");

    if let NodeType::Element { ref tag_name, ref attributes, .. } = sn_borrow.node_type {
        assert_eq!(tag_name, "select");
        assert_eq!(attributes.get("name").map(|s| s.as_str()), Some("country"));
    } else {
        panic!("Expected select to be an element");
    }
}

#[test]
fn test_textarea_dom_and_layout() {
    let html = r#"
        <html>
            <body>
                <textarea id="comments" rows="5" cols="40" style="width: 300px; height: 120px;">
                    Default feedback text here
                </textarea>
            </body>
        </html>
    "#;

    let mut parser = HTMLParser::new();
    let root = parser.parse(html);

    let ta_node = find_element_by_id(&root, "comments");
    assert!(ta_node.is_some());

    let layout = build_layout_tree(&root, "http://localhost/");
    assert!(layout.is_some());
}

#[test]
fn test_form_inputs_css_pseudoclasses() {
    let css = r#"
        input:checked {
            background-color: rgb(0, 255, 0);
        }
        input:disabled {
            opacity: 0.5;
        }
        select:required {
            border: 2px solid red;
        }
    "#;

    let mut parser = CSSParser::new(css);
    let stylesheet = parser.parse();
    assert_eq!(stylesheet.rules.len(), 3);
}

#[test]
fn test_nested_form_table_structure() {
    let html = r#"
        <html>
            <body>
                <form id="order-form">
                    <table>
                        <tr>
                            <th>Item</th>
                            <th>Quantity</th>
                            <th>Action</th>
                        </tr>
                        <tr>
                            <td>Assam Silk Scarf</td>
                            <td><input type="number" value="2" /></td>
                            <td><input type="checkbox" checked /></td>
                        </tr>
                    </table>
                </form>
            </body>
        </html>
    "#;

    let mut parser = HTMLParser::new();
    let root = parser.parse(html);
    let form_node = find_element_by_id(&root, "order-form");
    assert!(form_node.is_some());

    let layout = build_layout_tree(&root, "http://localhost/");
    assert!(layout.is_some(), "Layout tree should be generated for form with table");
}
