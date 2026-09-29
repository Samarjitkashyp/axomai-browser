import unittest
from browser.rust_bridge import is_rust_available

class TestRustBridge(unittest.TestCase):

    def test_rust_bridge_availability(self):
        # Checks that the bridge initializes without crashing
        available = is_rust_available()
        self.assertIn(available, [True, False])

if __name__ == "__main__":
    unittest.main()
