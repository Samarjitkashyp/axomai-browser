import unittest
from browser.html_parser import HTMLParser, Element
from browser.css_parser import CSSParser, DEFAULT_UA_STYLES, style_tree

class TestCSSParser(unittest.TestCase):

    def test_css_parsing(self):
        css = """
        h1 { color: red; font-size: 32px; }
        .container { background-color: blue; }
        #main p { color: green; }
        """
        parser = CSSParser(css)
        rules = parser.parse()
        self.assertEqual(len(rules), 3)

        self.assertEqual(rules[0].declarations["color"], "red")
        self.assertEqual(rules[0].declarations["font-size"], "32px")

    def test_style_resolution(self):
        html = '<div id="main"><p class="highlight" style="font-weight: bold;">Hello</p></div>'
        html_node = HTMLParser(html).parse()

        author_css = "p.highlight { color: red; } #main p { font-size: 20px; }"
        ua_rules = CSSParser(DEFAULT_UA_STYLES).parse()
        author_rules = CSSParser(author_css).parse()

        all_rules = ua_rules + author_rules
        style_tree(html_node, all_rules)

        p_node = html_node.children[0]
        self.assertEqual(p_node.tag, "p")
        self.assertEqual(p_node.style["color"], "red")
        self.assertEqual(p_node.style["font-size"], "20px")
        self.assertEqual(p_node.style["font-weight"], "bold")

if __name__ == "__main__":
    unittest.main()
