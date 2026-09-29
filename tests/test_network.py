import unittest
from browser.network import URL

class TestNetworkLayer(unittest.TestCase):

    def test_url_parsing(self):
        u1 = URL("http://example.com/index.html")
        self.assertEqual(u1.scheme, "http")
        self.assertEqual(u1.host, "example.com")
        self.assertEqual(u1.port, 80)
        self.assertEqual(u1.path, "/index.html")

        u2 = URL("https://example.com:8443/test")
        self.assertEqual(u2.scheme, "https")
        self.assertEqual(u2.host, "example.com")
        self.assertEqual(u2.port, 8443)
        self.assertEqual(u2.path, "/test")

    def test_data_url(self):
        u = URL("data:text/html,<h1>Hello</h1>")
        headers, body = u.request()
        self.assertEqual(body, "<h1>Hello</h1>")

    def test_https_request(self):
        u = URL("https://example.org/")
        headers, body = u.request()
        self.assertIn("example", body.lower())
        self.assertTrue(len(body) > 0)

if __name__ == "__main__":
    unittest.main()
