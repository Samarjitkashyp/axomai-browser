"""
Rust Native Extension Bridge for Axomai Browser.
Loads compiled Rust DLL/so native module when present, else falls back cleanly to Python engine.
"""

import os
import ctypes

RUST_ENGINE_AVAILABLE = False
_rust_lib = None

# Attempt loading compiled Rust DLL / shared library if present
rust_dll_paths = [
    os.path.join(os.path.dirname(__file__), "..", "native", "rust_engine", "target", "release", "axomai_engine.dll"),
    os.path.join(os.path.dirname(__file__), "..", "native", "rust_engine", "target", "debug", "axomai_engine.dll"),
]

for dll_path in rust_dll_paths:
    if os.path.exists(dll_path):
        try:
            _rust_lib = ctypes.CDLL(dll_path)
            _rust_lib.rust_parse_css_count.argtypes = [ctypes.c_char_p]
            _rust_lib.rust_parse_css_count.restype = ctypes.c_size_t
            RUST_ENGINE_AVAILABLE = True
            print(f"[Axomai Native]: Loaded Rust Engine from {dll_path}")
            break
        except Exception as e:
            print(f"[Axomai Native]: Could not load Rust DLL: {e}")

def is_rust_available() -> bool:
    return RUST_ENGINE_AVAILABLE
