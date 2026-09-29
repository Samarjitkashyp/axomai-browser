import unittest
from browser.html_parser import HTMLParser, Element
from browser.js_engine import JSEngine

class TestJSEngine(unittest.TestCase):

    def test_js_variables_and_console(self):
        html = "<html><body><h1>Test</h1></body></html>"
        dom_root = HTMLParser(html).parse()
        js = 'var msg = "Hello from JS"; console.log(msg);'

        engine = JSEngine(dom_root)
        mutated = engine.execute(js)

        self.assertFalse(mutated)
        self.assertEqual(engine.variables["msg"], "Hello from JS")
        self.assertEqual(engine.console_logs[0], "Hello from JS")

    def test_document_write_dom_mutation(self):
        html = "<html><body><div id='app'></div></body></html>"
        dom_root = HTMLParser(html).parse()
        js = 'var text = "Dynamic Content"; document.write("<p>" + text + "</p>");'

        engine = JSEngine(dom_root)
        mutated = engine.execute(js)

        self.assertTrue(mutated)
        body = dom_root.children[0]
        self.assertEqual(len(body.children), 2)
        p_child = body.children[1]
        self.assertEqual(p_child.tag, "p")
        self.assertEqual(p_child.children[0].text, "Dynamic Content")

if __name__ == "__main__":
    unittest.main()
