import unittest
from browser.html_parser import HTMLParser
from browser.css_parser import CSSParser, DEFAULT_UA_STYLES, style_tree
from browser.layout import build_layout_tree
from browser.painter import build_display_list, DrawInput, DrawButton

class TestFormControls(unittest.TestCase):

    def test_input_and_button_layout_and_display(self):
        html = '<html><body><input placeholder="Search..." value="hello" /><button>Submit</button></body></html>'
        dom_root = HTMLParser(html).parse()

        ua_rules = CSSParser(DEFAULT_UA_STYLES).parse()
        style_tree(dom_root, ua_rules)

        layout_root = build_layout_tree(dom_root)
        self.assertIsNotNone(layout_root)

        display_list = build_display_list(layout_root)
        input_cmds = [cmd for cmd in display_list if isinstance(cmd, DrawInput)]
        button_cmds = [cmd for cmd in display_list if isinstance(cmd, DrawButton)]

        self.assertEqual(len(input_cmds), 1)
        self.assertEqual(input_cmds[0].value, "hello")
        self.assertEqual(input_cmds[0].placeholder, "Search...")

        self.assertEqual(len(button_cmds), 1)
        self.assertEqual(button_cmds[0].label, "Submit")

if __name__ == "__main__":
    unittest.main()
