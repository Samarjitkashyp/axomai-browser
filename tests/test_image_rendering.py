import unittest
import base64
from browser.network import URL
from browser.html_parser import HTMLParser
from browser.css_parser import CSSParser, DEFAULT_UA_STYLES, style_tree
from browser.layout import build_layout_tree
from browser.painter import build_display_list, DrawImage

# Tiny 1x1 red PNG pixel
TINY_RED_PNG_B64 = "data:image/png;base64,iVBORw0KGgoAAAANSU5QAAAABJRU5ErkJggg=="

class TestImageRendering(unittest.TestCase):

    def test_data_url_binary_fetch(self):
        u = URL(TINY_RED_PNG_B64)
        headers, data_bytes = u.request_bytes()
        self.assertTrue(len(data_bytes) > 0)

    def test_image_element_layout_and_display(self):
        html = f'<html><body><img src="{TINY_RED_PNG_B64}" width="100" height="100" /></body></html>'
        url_obj = URL("data:text/html," + html)
        dom_root = HTMLParser(html).parse()

        ua_rules = CSSParser(DEFAULT_UA_STYLES).parse()
        style_tree(dom_root, ua_rules)

        layout_root = build_layout_tree(dom_root, url_obj)
        self.assertIsNotNone(layout_root)

        display_list = build_display_list(layout_root)
        image_cmds = [cmd for cmd in display_list if isinstance(cmd, DrawImage)]

        self.assertEqual(len(image_cmds), 1)
        self.assertEqual(image_cmds[0].width, 100.0)
        self.assertEqual(image_cmds[0].height, 100.0)
        self.assertTrue(len(image_cmds[0].image_bytes) > 0)

if __name__ == "__main__":
    unittest.main()
