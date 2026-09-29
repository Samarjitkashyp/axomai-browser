import html

class Node:
    def __init__(self, parent=None):
        self.parent = parent
        self.children = []

    def add_child(self, child):
        child.parent = self
        self.children.append(child)


class Text(Node):
    def __init__(self, text: str, parent=None):
        super().__init__(parent)
        self.text = text

    def __repr__(self):
        return f"Text({repr(self.text)})"


class Element(Node):
    def __init__(self, tag: str, attributes: dict[str, str] = None, parent=None):
        super().__init__(parent)
        self.tag = tag.lower()
        self.attributes = attributes if attributes is not None else {}
        self.style = {}  # Computed styles populated by Phase 3

    def __repr__(self):
        attrs = " ".join(f'{k}="{v}"' for k, v in self.attributes.items())
        return f"Element(<{self.tag}{' ' + attrs if attrs else ''}>)"


SELF_CLOSING_TAGS = {
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "param", "source", "track", "wbr"
}


class HTMLParser:
    def __init__(self, body: str):
        self.body = body
        self.unfinished = []

    def parse(self) -> Node:
        text = ""
        in_tag = False
        in_comment = False
        i = 0
        n = len(self.body)

        while i < n:
            c = self.body[i]

            # Check for comments
            if not in_tag and self.body.startswith("<!--", i):
                if text:
                    self.add_text(text)
                    text = ""
                end_comment = self.body.find("-->", i + 4)
                if end_comment != -1:
                    i = end_comment + 3
                else:
                    i = n
                continue

            # Check for script/style contents
            if self.unfinished:
                top_tag = self.unfinished[-1].tag
                if top_tag in ["script", "style"] and not self.body.startswith(f"</{top_tag}>", i):
                    end_raw = self.body.find(f"</{top_tag}>", i)
                    if end_raw != -1:
                        raw_content = self.body[i:end_raw]
                        self.add_text(raw_content)
                        i = end_raw
                        continue
                    else:
                        raw_content = self.body[i:]
                        self.add_text(raw_content)
                        break

            if c == "<":
                in_tag = True
                if text:
                    self.add_text(text)
                    text = ""
            elif c == ">":
                in_tag = False
                self.add_tag(text)
                text = ""
            else:
                text += c

            i += 1

        if not in_tag and text:
            self.add_text(text)

        return self.finish()

    def add_text(self, text: str):
        # Decode HTML entities (e.g. &lt; -> <)
        decoded = html.unescape(text)
        if not decoded:
            return
        if self.unfinished:
            self.unfinished[-1].add_child(Text(decoded))

    def add_tag(self, tag_content: str):
        tag_content = tag_content.strip()
        if not tag_content:
            return

        # Ignore doctype <!DOCTYPE ...>
        if tag_content.startswith("!") or tag_content.startswith("?"):
            return

        # Closing tag </p>
        if tag_content.startswith("/"):
            tag_name = tag_content[1:].strip().lower()
            if len(self.unfinished) > 1:
                # Find matching tag in stack
                for idx in range(len(self.unfinished) - 1, -1, -1):
                    if self.unfinished[idx].tag == tag_name:
                        # Pop everything off the stack down to and including this tag
                        while len(self.unfinished) > idx:
                            self.unfinished.pop()
                        break
            return

        # Open or self-closing tag
        tag_name, attributes = self.parse_attributes(tag_content)
        is_self_closing = tag_content.endswith("/") or tag_name in SELF_CLOSING_TAGS

        node = Element(tag_name, attributes)

        if is_self_closing:
            if self.unfinished:
                self.unfinished[-1].add_child(node)
            else:
                self.unfinished.append(node)
        else:
            if self.unfinished:
                self.unfinished[-1].add_child(node)
            self.unfinished.append(node)

    def parse_attributes(self, text: str) -> tuple[str, dict[str, str]]:
        parts = text.split(None, 1)
        tag_name = parts[0].rstrip("/").lower()
        if len(parts) == 1:
            return tag_name, {}

        attr_str = parts[1].rstrip("/")
        attributes = {}
        i = 0
        n = len(attr_str)

        while i < n:
            # Skip whitespace
            while i < n and attr_str[i].isspace():
                i += 1
            if i >= n:
                break

            # Read key
            key_start = i
            while i < n and not attr_str[i].isspace() and attr_str[i] != "=":
                i += 1
            key = attr_str[key_start:i].lower()

            # Skip whitespace around =
            while i < n and attr_str[i].isspace():
                i += 1

            if i < n and attr_str[i] == "=":
                i += 1
                while i < n and attr_str[i].isspace():
                    i += 1
                if i < n:
                    if attr_str[i] in ['"', "'"]:
                        quote = attr_str[i]
                        i += 1
                        val_start = i
                        while i < n and attr_str[i] != quote:
                            i += 1
                        val = attr_str[val_start:i]
                        if i < n:
                            i += 1  # consume closing quote
                    else:
                        val_start = i
                        while i < n and not attr_str[i].isspace():
                            i += 1
                        val = attr_str[val_start:i]
                    attributes[key] = val
                else:
                    attributes[key] = ""
            else:
                attributes[key] = ""

        return tag_name, attributes

    def finish(self) -> Node:
        if self.unfinished:
            return self.unfinished[0]

        # Fallback root node if body was completely empty
        root = Element("html")
        root.add_child(Element("body"))
        return root


def print_tree(node: Node, indent: int = 0):
    """Helper function to print DOM tree with indentation."""
    space = "  " * indent
    if isinstance(node, Element):
        attrs = " ".join(f'{k}="{v}"' for k, v in node.attributes.items())
        print(f"{space}<{node.tag}{' ' + attrs if attrs else ''}>")
        for child in node.children:
            print_tree(child, indent + 1)
        print(f"{space}</{node.tag}>")
    elif isinstance(node, Text):
        cleaned = node.text.strip().replace("\n", " ")
        if cleaned:
            print(f"{space}{cleaned}")
