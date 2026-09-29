from browser.layout import LayoutBox
from browser.html_parser import Element, Text

class DrawRect:
    def __init__(self, x1: float, y1: float, x2: float, y2: float, color: str):
        self.x1 = x1
        self.y1 = y1
        self.x2 = x2
        self.y2 = y2
        self.color = color

    def execute(self, canvas, scroll_y: float):
        canvas.create_rectangle(
            self.x1, self.y1 - scroll_y,
            self.x2, self.y2 - scroll_y,
            fill=self.color, outline=""
        )


class DrawText:
    def __init__(self, x: float, y: float, text: str, font, color: str):
        self.x = x
        self.y = y
        self.text = text
        self.font = font
        self.color = color

    def execute(self, canvas, scroll_y: float):
        canvas.create_text(
            self.x, self.y - scroll_y,
            text=self.text, font=self.font,
            fill=self.color, anchor="nw"
        )


def build_display_list(box: LayoutBox, display_list: list = None) -> list:
    if display_list is None:
        display_list = []

    if not box:
        return display_list

    # 1. Paint element background if present
    bg_color = box.style.get("background-color", None)
    if bg_color and bg_color != "transparent":
        display_list.append(DrawRect(
            box.x, box.y,
            box.x + box.width, box.y + box.height,
            bg_color
        ))

    # 2. Paint text nodes
    if box.box_type == "text" and box.word:
        color = box.style.get("color", "black")
        display_list.append(DrawText(
            box.x, box.y,
            box.word, box.font, color
        ))

    # 3. Recursively paint child boxes
    for child in box.children:
        build_display_list(child, display_list)

    return display_list
