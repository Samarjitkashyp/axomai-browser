import tkinter.font as tkfont
from browser.html_parser import Node, Element, Text

FONTS = {}

def get_font(size: int, weight: str = "normal", style: str = "normal", family: str = "sans-serif"):
    key = (size, weight, style, family)
    if key not in FONTS:
        weight_val = "bold" if weight == "bold" else "normal"
        slant_val = "italic" if style == "italic" else "roman"
        font_family = "Helvetica" if family == "sans-serif" else family
        FONTS[key] = tkfont.Font(family=font_family, size=int(size), weight=weight_val, slant=slant_val)
    return FONTS[key]


def parse_px(val: str, default: float = 0.0) -> float:
    if not val:
        return default
    val = val.strip().lower()
    if val.endswith("px"):
        try:
            return float(val[:-2])
        except ValueError:
            return default
    try:
        return float(val)
    except ValueError:
        return default


def get_href(node: Node) -> str:
    curr = node
    while curr:
        if isinstance(curr, Element) and curr.tag == "a":
            return curr.attributes.get("href", "")
        curr = getattr(curr, "parent", None)
    return ""


class LayoutBox:
    def __init__(self, node: Node, box_type: str = "block"):
        self.node = node
        self.box_type = box_type  # "block", "inline", "text"
        self.x = 0.0
        self.y = 0.0
        self.width = 0.0
        self.height = 0.0
        self.children = []
        self.word = ""  # for text boxes
        self.font = None
        self.style = getattr(node, "style", {})
        self.href = get_href(node)

    def layout(self, x: float, y: float, max_width: float) -> float:
        """
        Calculates geometry for this box and its children.
        Returns total height consumed by this box.
        """
        self.x = x
        self.y = y

        if self.box_type == "block":
            self.width = max_width
            margin_top = parse_px(self.style.get("margin-top", "0px"))
            margin_bottom = parse_px(self.style.get("margin-bottom", "0px"))
            padding_top = parse_px(self.style.get("padding-top", self.style.get("padding", "0px")))
            padding_bottom = parse_px(self.style.get("padding-bottom", self.style.get("padding", "0px")))
            padding_left = parse_px(self.style.get("padding-left", self.style.get("padding", "0px")))
            padding_right = parse_px(self.style.get("padding-right", self.style.get("padding", "0px")))

            content_width = max(0.0, self.width - padding_left - padding_right)
            cursor_y = y + margin_top + padding_top
            child_x = x + padding_left

            # Collect inline/text elements into line wrapper boxes or recursive blocks
            line_boxes = []
            for child in self.children:
                if child.box_type == "block":
                    if line_boxes:
                        cursor_y += self._layout_inline_lines(line_boxes, child_x, cursor_y, content_width)
                        line_boxes = []
                    child_height = child.layout(child_x, cursor_y, content_width)
                    cursor_y += child_height
                else:
                    line_boxes.append(child)

            if line_boxes:
                cursor_y += self._layout_inline_lines(line_boxes, child_x, cursor_y, content_width)

            self.height = (cursor_y - y) + margin_bottom + padding_bottom
            return self.height

        return 0.0

    def _layout_inline_lines(self, inline_children: list["LayoutBox"], x: float, y: float, max_width: float) -> float:
        cursor_x = x
        cursor_y = y
        line_height = 20.0

        for child in inline_children:
            if child.box_type == "text":
                font_size = parse_px(child.style.get("font-size", "16px"), 16.0)
                font_weight = child.style.get("font-weight", "normal")
                font_style = child.style.get("font-style", "normal")
                font_family = child.style.get("font-family", "sans-serif")
                font = get_font(int(font_size), font_weight, font_style, font_family)
                child.font = font
                line_height = max(line_height, font.metrics("linespace") * 1.2)

                words = child.word.split(" ")
                for idx, word in enumerate(words):
                    space = " " if idx < len(words) - 1 else ""
                    word_str = word + space
                    w = font.measure(word_str)

                    if cursor_x + w > x + max_width and cursor_x > x:
                        # Wrap line
                        cursor_x = x
                        cursor_y += line_height

                    word_box = LayoutBox(child.node, "text")
                    word_box.word = word_str
                    word_box.x = cursor_x
                    word_box.y = cursor_y
                    word_box.width = w
                    word_box.height = line_height
                    word_box.font = font
                    word_box.style = child.style
                    word_box.href = child.href
                    child.children.append(word_box)

                    cursor_x += w

        return (cursor_y - y) + line_height


def build_layout_tree(node: Node, current_url_obj=None) -> LayoutBox:
    if isinstance(node, Element):
        display = node.style.get("display", "block")
        if display == "none":
            return None

        if node.tag == "img":
            img_box = LayoutBox(node, "image")
            src = node.attributes.get("src", "")
            if src and current_url_obj:
                full_src = current_url_obj.resolve(src)
                try:
                    from browser.network import URL
                    headers, img_bytes = URL(full_src).request_bytes()
                    img_box.image_bytes = img_bytes
                except Exception:
                    img_box.image_bytes = b""
            w_str = node.attributes.get("width", node.style.get("width", "200px"))
            h_str = node.attributes.get("height", node.style.get("height", "150px"))
            img_box.width = parse_px(w_str, 200.0)
            img_box.height = parse_px(h_str, 150.0)
            return img_box

        box_type = "block" if display in ["block", "flex", "table"] else "inline"
        root_box = LayoutBox(node, box_type)

        for child in node.children:
            child_box = build_layout_tree(child, current_url_obj)
            if child_box:
                root_box.children.append(child_box)

        return root_box

    elif isinstance(node, Text):
        cleaned_text = node.text.replace("\n", " ").strip()
        if not cleaned_text:
            return None
        text_box = LayoutBox(node, "text")
        text_box.word = cleaned_text
        text_box.style = getattr(node.parent, "style", {})
        return text_box

    return None
