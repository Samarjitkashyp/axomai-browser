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


def draw_rounded_rect(canvas, x1, y1, x2, y2, radius=14, fill="", outline="", width=1):
    points = [
        x1 + radius, y1,
        x2 - radius, y1,
        x2, y1,
        x2, y1 + radius,
        x2, y2 - radius,
        x2, y2,
        x2 - radius, y2,
        x1 + radius, y2,
        x1, y2,
        x1, y2 - radius,
        x1, y1 + radius,
        x1, y1
    ]
    return canvas.create_polygon(points, fill=fill, outline=outline, width=width, smooth=True)


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
        border_color = "#1a73e8" if self.is_focused else "#dfe1e5"
        bg_color = "#ffffff" if self.is_focused else "#f8f9fa"
        radius = min(18, self.height / 2)
        
        # Chrome Pill Search Bar Box
        draw_rounded_rect(
            canvas,
            self.x, self.y - scroll_y,
            self.x + self.width, self.y + self.height - scroll_y,
            radius=radius, fill=bg_color, outline=border_color, width=2 if self.is_focused else 1
        )
        
        # Search icon prefix
        canvas.create_text(
            self.x + 12, self.y + (self.height / 2) - scroll_y,
            text="🔍", font=("sans-serif", 9), fill="#5f6368", anchor="w"
        )
        
        display_text = self.value if self.value else self.placeholder
        text_color = "#202124" if self.value else "#80868b"
        if self.is_focused:
            display_text += "|"
            
        if display_text:
            canvas.create_text(
                self.x + 30, self.y + (self.height / 2) - scroll_y,
                text=display_text, fill=text_color, anchor="w",
                font=("Segoe UI", 10)
            )


class DrawButton:
    def __init__(self, x: float, y: float, width: float, height: float, label: str):
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.label = label

    def execute(self, canvas, scroll_y: float):
        radius = min(16, self.height / 2)
        draw_rounded_rect(
            canvas,
            self.x, self.y - scroll_y,
            self.x + self.width, self.y + self.height - scroll_y,
            radius=radius, fill="#1a73e8", outline="#174ea6", width=1
        )
        canvas.create_text(
            self.x + (self.width / 2), self.y + (self.height / 2) - scroll_y,
            text=self.label, fill="#ffffff", anchor="center",
            font=("Segoe UI", 10, "bold")
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
