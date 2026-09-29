import socket
import ssl

class URL:
    def __init__(self, url_string: str):
        self.scheme = "http"
        self.host = ""
        self.port = None
        self.path = "/"

        # Handle scheme matching strictly at the start of the URL string
        if url_string.startswith("https://"):
            self.scheme = "https"
            url_string = url_string[8:]
        elif url_string.startswith("http://"):
            self.scheme = "http"
            url_string = url_string[7:]
        elif url_string.startswith("file://"):
            self.scheme = "file"
            self.path = url_string[7:]
            return
        elif url_string.startswith("data:"):
            self.scheme = "data"
            self.path = url_string[5:]
            return
        elif ":" in url_string:
            scheme_part, rest = url_string.split(":", 1)
            if scheme_part.lower() in ["http", "https", "file", "data"]:
                self.scheme = scheme_part.lower()
                url_string = rest
                if self.scheme == "data":
                    self.path = rest
                    return
                elif self.scheme == "file":
                    self.path = rest
                    return

        if self.scheme not in ["http", "https", "file", "data"]:
            raise ValueError(f"Unsupported scheme: {self.scheme}")

        # Separate host and path
        if "/" in url_string:
            self.host, self.path = url_string.split("/", 1)
            self.path = "/" + self.path
        else:
            self.host = url_string
            self.path = "/"

        # Handle custom port in host
        if ":" in self.host:
            self.host, port_str = self.host.split(":", 1)
            self.port = int(port_str)
        else:
            if self.scheme == "http":
                self.port = 80
            elif self.scheme == "https":
                self.port = 443

    def resolve(self, relative_url: str) -> str:
        """Resolves relative URL path against this URL's base location."""
        if not relative_url:
            return f"{self.scheme}://{self.host}{self.path}"
        if "://" in relative_url or relative_url.startswith("data:") or relative_url.startswith("file:"):
            return relative_url
        if relative_url.startswith("//"):
            return f"{self.scheme}:{relative_url}"
        
        port_suffix = f":{self.port}" if self.port and self.port not in [80, 443] else ""
        if relative_url.startswith("/"):
            return f"{self.scheme}://{self.host}{port_suffix}{relative_url}"
        
        dir_path = self.path.rsplit("/", 1)[0]
        if not dir_path.startswith("/"):
            dir_path = "/" + dir_path
        full_path = f"{dir_path}/{relative_url}".replace("//", "/")
        return f"{self.scheme}://{self.host}{port_suffix}{full_path}"

    def request(self) -> tuple[dict[str, str], str]:
        """
        Fetches the URL over HTTP/HTTPS or local file/data scheme.
        Returns a tuple of (response_headers, response_body_str).
        """
        if self.scheme == "file":
            try:
                with open(self.path, "r", encoding="utf-8") as f:
                    return {}, f.read()
            except Exception as e:
                return {}, f"<html><body><h1>Error loading file</h1><p>{e}</p></body></html>"

        if self.scheme == "data":
            # Simple data URL e.g. data:text/html,Hello World
            if "," in self.path:
                header, body = self.path.split(",", 1)
                return {"content-type": header}, body
            return {}, self.path

        # Create TCP Connection
        s = socket.create_connection((self.host, self.port), timeout=10)
        response_bytes = bytearray()
        try:
            # Wrap in SSL for HTTPS
            if self.scheme == "https":
                ctx = ssl.create_default_context()
                s = ctx.wrap_socket(s, server_hostname=self.host)

            # Formulate HTTP GET request
            request_data = (
                f"GET {self.path} HTTP/1.1\r\n"
                f"Host: {self.host}\r\n"
                f"User-Agent: AxomaiBrowser/1.0\r\n"
                f"Connection: close\r\n"
                f"Accept: text/html,application/xhtml+xml\r\n"
                f"\r\n"
            )
            s.sendall(request_data.encode("utf-8"))

            # Read response into buffer loop
            while True:
                chunk = s.recv(4096)
                if not chunk:
                    break
                response_bytes.extend(chunk)
        finally:
            s.close()

        # Parse status line, headers, and body
        return self._parse_response(response_bytes)

    def _parse_response(self, response_bytes: bytearray) -> tuple[dict[str, str], str]:
        # Split headers and body at \r\n\r\n or \n\n
        delimiter = b"\r\n\r\n"
        if delimiter in response_bytes:
            header_bytes, body_bytes = response_bytes.split(delimiter, 1)
        elif b"\n\n" in response_bytes:
            header_bytes, body_bytes = response_bytes.split(b"\n\n", 1)
        else:
            header_bytes = response_bytes
            body_bytes = b""

        header_lines = header_bytes.decode("iso-8859-1", errors="replace").splitlines()
        headers = {}
        status_line = header_lines[0] if header_lines else ""

        for line in header_lines[1:]:
            if ":" in line:
                key, val = line.split(":", 1)
                headers[key.strip().lower()] = val.strip()

        # Decode body (handling utf-8 with fallback)
        body = body_bytes.decode("utf-8", errors="replace")
        return headers, body

    def request_bytes(self) -> tuple[dict[str, str], bytes]:
        """Fetches raw binary data (e.g. images) from URL."""
        if self.scheme == "file":
            try:
                with open(self.path, "rb") as f:
                    return {}, f.read()
            except Exception:
                return {}, b""

        if self.scheme == "data":
            if "," in self.path:
                header, body_str = self.path.split(",", 1)
                if ";base64" in header:
                    import base64
                    try:
                        return {"content-type": header}, base64.b64decode(body_str)
                    except Exception:
                        return {"content-type": header}, b""
                return {"content-type": header}, body_str.encode("utf-8")
            return {}, self.path.encode("utf-8")

        s = socket.create_connection((self.host, self.port), timeout=10)
        response_bytes = bytearray()
        try:
            if self.scheme == "https":
                ctx = ssl.create_default_context()
                s = ctx.wrap_socket(s, server_hostname=self.host)

            request_data = (
                f"GET {self.path} HTTP/1.1\r\n"
                f"Host: {self.host}\r\n"
                f"User-Agent: AxomaiBrowser/1.0\r\n"
                f"Connection: close\r\n"
                f"Accept: image/*,*/*\r\n"
                f"\r\n"
            )
            s.sendall(request_data.encode("utf-8"))

            while True:
                chunk = s.recv(4096)
                if not chunk:
                    break
                response_bytes.extend(chunk)
        finally:
            s.close()

        delimiter = b"\r\n\r\n"
        if delimiter in response_bytes:
            header_bytes, body_bytes = response_bytes.split(delimiter, 1)
        elif b"\n\n" in response_bytes:
            header_bytes, body_bytes = response_bytes.split(b"\n\n", 1)
        else:
            header_bytes = response_bytes
            body_bytes = b""

        header_lines = header_bytes.decode("iso-8859-1", errors="replace").splitlines()
        headers = {}
        for line in header_lines[1:]:
            if ":" in line:
                key, val = line.split(":", 1)
                headers[key.strip().lower()] = val.strip()

        return headers, bytes(body_bytes)


def fetch(url_str: str) -> tuple[dict[str, str], str]:
    """Helper function to fetch URL directly."""
    url = URL(url_str)
    return url.request()
