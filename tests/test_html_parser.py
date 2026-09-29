import unittest
from browser.html_parser import HTMLParser, Element, Text

class TestHTMLParser(unittest.TestCase):

    def test_simple_html(self):
        html_content = "<html><body><h1>Title</h1><p>Hello <b>World</b>!</p></body></html>"
        parser = HTMLParser(html_content)
        root = parser.parse()

        self.assertIsInstance(root, Element)
        self.assertEqual(root.tag, "html")
        self.assertEqual(len(root.children), 1)

        body = root.children[0]
        self.assertEqual(body.tag, "body")
        self.assertEqual(len(body.children), 2)

        h1 = body.children[0]
        self.assertEqual(h1.tag, "h1")
        self.assertEqual(h1.children[0].text, "Title")

    def test_attributes_and_self_closing(self):
        html_content = '<div id="main" class="container"><img src="test.jpg" /><br></div>'
        parser = HTMLParser(html_content)
        root = parser.parse()

        self.assertEqual(root.tag, "div")
        self.assertEqual(root.attributes["id"], "main")
        self.assertEqual(root.attributes["class"], "container")
        self.assertEqual(len(root.children), 2)
        self.assertEqual(root.children[0].tag, "img")
        self.assertEqual(root.children[1].tag, "br")

if __name__ == "__main__":
    unittest.main()
