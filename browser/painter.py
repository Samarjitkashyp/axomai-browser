import io
from PIL import Image, ImageTk
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
    def __init__(self, x: float, y: float, width: float, height: float, text: str, font, color: str, href: str = ""):
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.text = text
        self.font = font
        self.color = color
        self.href = href

    def execute(self, canvas, scroll_y: float):
        canvas.create_text(
            self.x, self.y - scroll_y,
            text=self.text, font=self.font,
            fill=self.color, anchor="nw"
        )


class DrawImage:
    def __init__(self, x: float, y: float, width: float, height: float, image_bytes: bytes):
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.image_bytes = image_bytes
        self._tk_photo = None

    def execute(self, canvas, scroll_y: float):
        if not self._tk_photo and self.image_bytes:
            try:
                pil_img = Image.open(io.BytesIO(self.image_bytes))
                w = int(self.width) if self.width > 0 else pil_img.width
                h = int(self.height) if self.height > 0 else pil_img.height
                if w > 0 and h > 0 and (w != pil_img.width or h != pil_img.height):
                    pil_img = pil_img.resize((w, h), Image.Resampling.LANCZOS)
                self._tk_photo = ImageTk.PhotoImage(pil_img)
            except Exception as e:
                print(f"[Painter Image Error]: {e}")
                return

        if self._tk_photo:
            canvas.create_image(
                self.x, self.y - scroll_y,
                image=self._tk_photo, anchor="nw"
            )


class DrawInput:
    def __init__(self, x: float, y: float, width: float, height: float, value: str, placeholder: str, is_focused: bool = False, node=None):
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.value = value
        self.placeholder = placeholder
        self.is_focused = is_focused
        self.node = node

    def execute(self, canvas, scroll_y: float):
        border_color = "#0284c7" if self.is_focused else "#cbd5e1"
        bg_color = "#ffffff"
        canvas.create_rectangle(
            self.x, self.y - scroll_y,
            self.x + self.width, self.y + self.height - scroll_y,
            fill=bg_color, outline=border_color, width=2 if self.is_focused else 1
        )
        display_text = self.value if self.value else self.placeholder
        text_color = "#0f172a" if self.value else "#94a3b8"
        if self.is_focused:
            display_text += "|"
        if display_text:
            canvas.create_text(
                self.x + 8, self.y + (self.height / 2) - scroll_y,
                text=display_text, fill=text_color, anchor="w",
                font=("sans-serif", 10)
            )


class DrawButton:
    def __init__(self, x: float, y: float, width: float, height: float, label: str):
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.label = label

    def execute(self, canvas, scroll_y: float):
        canvas.create_rectangle(
            self.x, self.y - scroll_y,
            self.x + self.width, self.y + self.height - scroll_y,
            fill="#0284c7", outline="#0369a1", width=1
        )
        canvas.create_text(
            self.x + (self.width / 2), self.y + (self.height / 2) - scroll_y,
            text=self.label, fill="#ffffff", anchor="center",
            font=("sans-serif", 10, "bold")
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

    # 2. Paint text nodes (only leaf text boxes)
    if box.box_type == "text" and box.word and not box.children:
        color = box.style.get("color", "black")
        display_list.append(DrawText(
            box.x, box.y, box.width, box.height,
            box.word, box.font, color, box.href
        ))

    # 3. Paint image nodes
    if box.box_type == "image" and getattr(box, "image_bytes", None):
        display_list.append(DrawImage(
            box.x, box.y, box.width, box.height,
            box.image_bytes
        ))

    # 4. Paint input fields
    if box.box_type == "input":
        display_list.append(DrawInput(
            box.x, box.y, box.width, box.height,
            getattr(box, "value", ""),
            getattr(box, "placeholder", ""),
            is_focused=getattr(box, "is_focused", False),
            node=box.node
        ))

    # 5. Paint buttons
    if box.box_type == "button":
        btn_label = "Submit"
        if box.children and hasattr(box.children[0], "word"):
            btn_label = box.children[0].word.strip()
        display_list.append(DrawButton(
            box.x, box.y, box.width, box.height,
            btn_label
        ))

    # 6. Recursively paint child boxes
    for child in box.children:
        build_display_list(child, display_list)

    return display_list
