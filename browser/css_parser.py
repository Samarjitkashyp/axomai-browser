from browser.html_parser import Node, Element, Text

class TagSelector:
    def __init__(self, tag: str):
        self.tag = tag.lower()

    def matches(self, node: Node) -> bool:
        return isinstance(node, Element) and node.tag == self.tag

    def __repr__(self):
        return f"TagSelector({self.tag})"


class ClassSelector:
    def __init__(self, class_name: str):
        self.class_name = class_name.lower()

    def matches(self, node: Node) -> bool:
        if not isinstance(node, Element):
            return False
        classes = node.attributes.get("class", "").lower().split()
        return self.class_name in classes

    def __repr__(self):
        return f"ClassSelector(.{self.class_name})"


class IDSelector:
    def __init__(self, id_name: str):
        self.id_name = id_name.lower()

    def matches(self, node: Node) -> bool:
        if not isinstance(node, Element):
            return False
        return node.attributes.get("id", "").lower() == self.id_name

    def __repr__(self):
        return f"IDSelector(#{self.id_name})"


class DescendantSelector:
    def __init__(self, ancestor_sel, descendant_sel):
        self.ancestor_sel = ancestor_sel
        self.descendant_sel = descendant_sel

    def matches(self, node: Node) -> bool:
        if not self.descendant_sel.matches(node):
            return False
        curr = node.parent
        while curr:
            if self.ancestor_sel.matches(curr):
                return True
            curr = curr.parent
        return False

    def __repr__(self):
        return f"DescendantSelector({self.ancestor_sel} {self.descendant_sel})"


class Rule:
    def __init__(self, selector, declarations: dict[str, str]):
        self.selector = selector
        self.declarations = declarations

    def __repr__(self):
        return f"Rule({self.selector} -> {self.declarations})"


class CSSParser:
    def __init__(self, css_text: str):
        self.css_text = css_text

    def parse(self) -> list[Rule]:
        rules = []
        i = 0
        n = len(self.css_text)

        while i < n:
            # Skip whitespace
            while i < n and self.css_text[i].isspace():
                i += 1
            if i >= n:
                break

            # Skip comments /* ... */
            if self.css_text.startswith("/*", i):
                end_comm = self.css_text.find("*/", i + 2)
                if end_comm != -1:
                    i = end_comm + 2
                else:
                    i = n
                continue

            # Read selector up to '{'
            sel_start = i
            while i < n and self.css_text[i] != "{":
                i += 1
            if i >= n:
                break

            sel_text = self.css_text[sel_start:i].strip()
            i += 1  # consume '{'

            # Read declarations up to '}'
            body_start = i
            while i < n and self.css_text[i] != "}":
                i += 1
            body_text = self.css_text[body_start:i].strip()
            if i < n:
                i += 1  # consume '}'

            if not sel_text:
                continue

            declarations = parse_declarations(body_text)
            if not declarations:
                continue

            # Split comma-separated selectors e.g. "head, script, style" or "h1, h2"
            for sub_sel in sel_text.split(","):
                sub_sel = sub_sel.strip()
                if sub_sel:
                    selector = parse_selector(sub_sel)
                    if selector:
                        rules.append(Rule(selector, declarations))

        return rules


class CompoundSelector:
    def __init__(self, selectors):
        self.selectors = selectors

    def matches(self, node: Node) -> bool:
        return all(sel.matches(node) for sel in self.selectors)

    def __repr__(self):
        return f"CompoundSelector({''.join(str(s) for s in self.selectors)})"


def parse_selector(sel_str: str):
    sel_str = sel_str.strip()
    parts = sel_str.split()
    if not parts:
        return None

    selectors = [parse_single_selector(part) for part in parts]

    if len(selectors) == 1:
        return selectors[0]
    
    current = selectors[0]
    for sel in selectors[1:]:
        current = DescendantSelector(current, sel)
    return current


def parse_single_selector(part: str):
    sub_selectors = []
    # Match id #
    if "#" in part:
        tag_or_class, id_part = part.split("#", 1)
        sub_selectors.append(IDSelector(id_part))
        part = tag_or_class

    # Match class .
    if "." in part:
        tag_part, class_part = part.split(".", 1)
        sub_selectors.append(ClassSelector(class_part))
        part = tag_part

    if part:
        sub_selectors.append(TagSelector(part))

    if len(sub_selectors) == 1:
        return sub_selectors[0]
    return CompoundSelector(sub_selectors)


def parse_declarations(body_str: str) -> dict[str, str]:
    declarations = {}
    for decl in body_str.split(";"):
        decl = decl.strip()
        if ":" in decl:
            prop, val = decl.split(":", 1)
            declarations[prop.strip().lower()] = val.strip().lower()
    return declarations


# Default User Agent Stylesheet
DEFAULT_UA_STYLES = """
html { display: block; color: black; background-color: white; }
body { display: block; margin: 8px; font-size: 16px; font-family: sans-serif; }
div { display: block; }
p { display: block; margin-top: 10px; margin-bottom: 10px; }
h1 { display: block; font-size: 32px; font-weight: bold; margin-top: 15px; margin-bottom: 15px; }
h2 { display: block; font-size: 24px; font-weight: bold; margin-top: 12px; margin-bottom: 12px; }
h3 { display: block; font-size: 18px; font-weight: bold; margin-top: 10px; margin-bottom: 10px; }
b, strong { display: inline; font-weight: bold; }
i, em { display: inline; font-style: italic; }
a { display: inline; color: blue; text-decoration: underline; }
span { display: inline; }
img { display: inline-block; }
head, script, style { display: none; }
"""

INHERITED_PROPERTIES = {"color", "font-size", "font-family", "font-weight", "font-style"}


def style_tree(node: Node, rules: list[Rule]):
    """Recursively resolves computed styles for each element in the DOM tree."""
    if isinstance(node, Element):
        node.style = {}

        # 1. Apply inherited styles from parent
        if node.parent and hasattr(node.parent, "style"):
            for prop in INHERITED_PROPERTIES:
                if prop in node.parent.style:
                    node.style[prop] = node.parent.style[prop]

        # 2. Apply matching CSS rules (User-Agent and Author)
        for rule in rules:
            if rule.selector.matches(node):
                for prop, val in rule.declarations.items():
                    node.style[prop] = val

        # 3. Apply inline styles (style="...")
        if "style" in node.attributes:
            inline_decls = parse_declarations(node.attributes["style"])
            for prop, val in inline_decls.items():
                node.style[prop] = val

    for child in node.children:
        style_tree(child, rules)
