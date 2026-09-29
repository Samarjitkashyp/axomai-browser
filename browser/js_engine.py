import re
from browser.html_parser import HTMLParser, Node, Element, Text

class JSEngine:
    def __init__(self, dom_root: Node):
        self.dom_root = dom_root
        self.variables = {}
        self.console_logs = []

    def execute(self, js_code: str) -> bool:
        """
        Evaluates a minimal subset of JavaScript:
        - console.log("text" or var)
        - document.write("html text" or var)
        - var / let / const assignment
        Returns True if DOM was mutated, requiring re-layout.
        """
        dom_mutated = False
        lines = [line.strip() for line in js_code.split(";") if line.strip()]

        for line in lines:
            # 1. console.log(...)
            match_log = re.match(r'console\.log\s*\((.*)\)', line)
            if match_log:
                val = self._eval_expr(match_log.group(1))
                print(f"[JS Console]: {val}")
                self.console_logs.append(str(val))
                continue

            # 2. document.write(...)
            match_write = re.match(r'document\.write\s*\((.*)\)', line)
            if match_write:
                html_snippet = str(self._eval_expr(match_write.group(1)))
                snippet_node = HTMLParser(html_snippet).parse()
                
                # Append snippet to <body> if present, else root
                target = self._find_body(self.dom_root) or self.dom_root
                if isinstance(snippet_node, Element) and snippet_node.tag == "html":
                    for child in snippet_node.children:
                        target.add_child(child)
                else:
                    target.add_child(snippet_node)
                dom_mutated = True
                continue

            # 3. Variable declaration var x = ... / let x = ...
            match_var = re.match(r'(?:var|let|const)\s+([a-zA-Z_]\w*)\s*=\s*(.*)', line)
            if match_var:
                var_name = match_var.group(1)
                var_val = self._eval_expr(match_var.group(2))
                self.variables[var_name] = var_val
                continue

        return dom_mutated

    def _eval_expr(self, expr_str: str):
        expr_str = expr_str.strip()
        
        # Handle string concatenation '+' outside quotes first
        if "+" in expr_str:
            parts = self._split_addition(expr_str)
            if len(parts) > 1:
                return "".join(str(self._eval_expr(p)) for p in parts)

        # String literal "..." or '...'
        if (expr_str.startswith('"') and expr_str.endswith('"')) or (expr_str.startswith("'") and expr_str.endswith("'")):
            return expr_str[1:-1]
        
        # Number literal
        try:
            if "." in expr_str:
                return float(expr_str)
            return int(expr_str)
        except ValueError:
            pass

        # Variable lookup
        if expr_str in self.variables:
            return self.variables[expr_str]

        return expr_str

    def _split_addition(self, expr: str) -> list[str]:
        parts = []
        in_quote = None
        curr = ""
        for c in expr:
            if c in ['"', "'"]:
                if in_quote is None:
                    in_quote = c
                elif in_quote == c:
                    in_quote = None
                curr += c
            elif c == "+" and in_quote is None:
                parts.append(curr)
                curr = ""
            else:
                curr += c
        if curr:
            parts.append(curr)
        return parts

    def _find_body(self, node: Node):
        if isinstance(node, Element) and node.tag == "body":
            return node
        for child in getattr(node, "children", []):
            found = self._find_body(child)
            if found:
                return found
        return None
