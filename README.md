# 🚀 Axomai Browser `v0.9.0`

> **Fast. Private. AI-Powered. Built for Everyone.**

A modern, high-performance web browser designed with an independent Rust engine core, Google V8 JavaScript/WebAssembly runtime, W3C-compliant Web Platform APIs, native windowing, and a glassmorphic desktop interface.

---

## 🏗️ Architecture Pipeline

```
[ URL ] ──► ( 1. Network Layer - Rust / HttpCache / CSP / CORS ) ──► [ Raw HTML/HTTP ]
                                                                        │
                                                                        ▼
                                                        ( 2. HTML5 DOM Parser - Rust )
                                                        - Tokenizer & Entity Decoder
                                                        - Implicit <tbody> Tree Builder
                                                                        │
                                                                        ▼
                                                                [ DOM AST Tree ]
                                                                        │
                                                                        ▼
                                                        ( 3. CSS Engine & Cascade - Rust )
                                                        - Custom Properties: var(--*)
                                                        - Math Expressions: calc(), min(), clamp()
                                                        - @keyframes & Transitions
                                                        - Specificity, !important & Pseudo-classes
                                                                        │
                                                                        ▼
                                                            [ Computed Style Tree ]
                                                                        │
                                                                        ▼
                                                        ( 4. Multi-model Layout Engine )
                                                        - Block & Inline Formatting
                                                        - Multi-pass Flexbox Engine
                                                        - 2D CSS Grid & Named Areas
                                                        - Table & Cell Auto-sizing
                                                        - Form Controls (Input, Select, Textarea)
                                                                        │
                                                                        ▼
                                                            [ Layout Box Tree ]
                                                                        │
                                                                        ▼
                                                        ( 5. Text Shaper & HarfBuzz - Rust )
                                                        - Indic (Assamese/Bengali) Ligatures
                                                        - Matra Reordering & Halant Conjuncts
                                                        - BiDi Directional Flow (Arabic/RTL)
                                                                        │
                                                                        ▼
                                                        ( 6. GPU Compositor & LayerTree )
                                                        - Layer Decomposition (Transform, Scroll, Fixed)
                                                        - Damage Region & Partial Repaint
                                                        - Direct GPU Vertex & Command Stream
                                                                        │
                                                                        ▼
                                            ┌───────────────────────────┴───────────────────────────┐
                                            ▼                                                       ▼
                            ( 7. Native Tao + Wry Desktop Host )                    ( 8. wgpu / Canvas Render Bridge )
```

---

## 📊 Subsystem Status Overview (`v0.9.0`)

| Subsystem | Status | Description |
| :--- | :---: | :--- |
| **V8 ECMAScript Engine** | 🟢 | Full Google V8 isolate, closures, microtasks, Promise lifecycle |
| **HTML5 Parser** | 🟢 | Tokenizer, entity decoding, implicit `<tbody>`, void tags, template parsing |
| **CSS Selectors** | 🟢 | `#id`, `.class`, tags, combinators (`>`, `+`, `~`, ` `), pseudo-classes (`:first-child`, `:last-child`, `:nth-child`, `:not`, `:disabled`, `:checked`) |
| **CSS Custom Properties** | 🟢 | `var(--name, fallback)` inheritance from `:root` and cascading ancestors |
| **CSS Math Expressions** | 🟢 | `calc()`, `min()`, `max()`, `clamp()` with units (`px`, `%`, `rem`, `em`, `pt`) |
| **CSS Animations & Transitions** | 🟢 | `@keyframes` timeline sampling, cubic bezier solver, RGBA/length interpolation |
| **Box Model & Layout** | 🟢 | Margins, borders, padding, content box sizing, inline word-wrapping |
| **Flexbox Engine** | 🟢 | Multi-pass measurement, flex-grow/shrink distribution, flex-wrap, align-content |
| **CSS Grid Layout** | 🟢 | `grid-template-columns/rows`, `grid-template-areas`, `auto-fit`/`auto-fill`, `minmax()`, alignments |
| **Table Layout** | 🟢 | `<table>`, `<tr>`, `<td>`, `<th>`, `<thead>`, `<tbody>`, `<tfoot>`, border-spacing |
| **GPU Compositor LayerTree** | 🟢 **NEW** | `LayerTree`, `LayerType` (Transform, Scroll, Fixed, Sticky), `DamageRegion` partial repaint, `GpuCommand` stream |
| **Indic & Assamese Text Shaper** | 🟢 **NEW** | `TextShaper` HarfBuzz engine, Matra reordering, Halant conjuncts (অসমীয়া, য-ফলা), BiDi Arabic RTL |
| **HTTP Cache & Validation** | 🟢 | `HttpCache`, `Cache-Control` (`max-age`, `no-store`, `no-cache`), `ETag` / `304 Not Modified` conditional revalidation |
| **Content Security Policy (CSP)** | 🟢 | `CspPolicy` (`default-src`, `script-src`, `style-src`, `connect-src`, `img-src`, `'self'`, `*`) |
| **ServiceWorker & PWAs** | 🟢 | `navigator.serviceWorker.register()`, `ServiceWorkerRegistration`, `ServiceWorkerContainer` |
| **Realtime & Messaging APIs** | 🟢 | `WebSocket` (full connection & state lifecycle), `BroadcastChannel` cross-context messaging |
| **Binary & File APIs** | 🟢 | `Blob` (`size`, `type`, `text()`, `slice()`), `File` (`lastModified`, `name`) |
| **Observers Subsystem** | 🟢 | `ResizeObserver`, `IntersectionObserver`, `MutationObserver` with full DOM mutation hooks |
| **Storage Subsystem** | 🟢 | `IndexedDB` (`open`, `IDBDatabase`, `IDBObjectStore`, `IDBTransaction`), `CacheStorage` (`window.caches`) |
| **Advanced Form Controls** | 🟢 | `<select>` + `<option>`, `<textarea>`, checkbox/radio groups mutual exclusion, `.click()` |
| **DOM Parser & Serializer** | 🟢 | `DOMParser` (`parseFromString`), `XMLSerializer` (`serializeToString`) |
| **DOM Geometry APIs** | 🟢 | `getBoundingClientRect()`, `offsetWidth/Height`, `offsetLeft/Top`, `clientWidth/Height` |
| **DOM Traversal & Mutation** | 🟢 | `matches()`, `closest()`, `contains()`, `cloneNode()`, `insertBefore()`, `replaceChild()`, `prepend()` |
| **W3C Event System** | 🟢 | Capturing, At-Target, Bubbling phases, Pointer capture, `preventDefault()` lifecycle |
| **Animation Clock** | 🟢 | `requestAnimationFrame()`, `cancelAnimationFrame()`, `performance.now()` |
| **Computed Style Proxy** | 🟢 | `window.getComputedStyle()` live reactive style reflection |
| **Web Workers** | 🟢 Foundation | Dedicated `Worker` constructor, thread message passing (`postMessage`, `onmessage`) |
| **Security & SOP / CORS** | 🟢 Active | Cross-origin verification, `Access-Control-Allow-Origin` enforcement |
| **Web Storage** | 🟢 | `localStorage`, `sessionStorage` (W3C Storage APIs + disk persistence) |
| **Form APIs** | 🟢 | `form.submit()`, `form.requestSubmit()`, `form.reset()`, `input.checkValidity()`, `FormData` |
| **CI Automated Verification** | 🟢 | Linux & Windows automated `cargo check` and `cargo test` on push & PR |

---

## 🛠️ Testing & Verification

Comprehensive automated test suites are provided under `native/rust_engine/tests/`:

```cmd
cd native/rust_engine
cargo test
```

### Test Coverage Suites:
- **`gpu_compositor_and_text_shaping_tests.rs`**: LayerTree decomposition, damage region intersection, GPU commands, Assamese (অসমীয়া), Hindi (नमस्कार) conjunct shaping, Arabic BiDi RTL.
- **`network_cache_and_serviceworker_tests.rs`**: `CacheControl` parsing, `CspPolicy` directive validation, base64 operations, URL origin resolution.
- **`observers_and_storage_tests.rs`**: Select & Option DOM parsing, Textarea layout, Form input pseudo-classes (`:checked`, `:disabled`), nested table forms.
- **`html_parser_tests.rs`**: Entity decoding (`&amp;`, `&lt;`), implicit `<tbody>` insertion, void tag self-closing.
- **`css_selector_tests.rs`**: Direct child (`>`), adjacent sibling (`+`), descendant combinators, attribute matching, `:first-child`, `:last-child`, `:disabled`.
- **`css_variables_and_calc_tests.rs`**: `:root` `--var` cascading and `var()` fallback resolution, `calc()`, `clamp()` mathematical evaluations.
- **`table_layout_tests.rs`**: Multi-row, multi-cell table auto-sizing and height balancing.
- **`dom_tests.rs`**: Multi-class selector matching, element ID retrieval, DOM hierarchy queries.

---

## 🗺️ Engineering Roadmap

- [x] **v0.5.4**: DOM persistent node identity, grid placement (`grid-column`/`grid-row`), `transform-origin`, `preventDefault` lifecycle.
- [x] **v0.5.5**: CSS Selector engine integration in `querySelector`/`querySelectorAll`, CSS Grid named areas & alignments, GitHub Actions CI.
- [x] **v0.5.6**: CSS transitions & `@keyframes` animation engine, `requestAnimationFrame`, `getComputedStyle`, Web Workers foundation, CORS enforcement.
- [x] **v0.6.0**: CSS Custom Properties (`var(--*)`), `calc()`/`min()`/`max()`/`clamp()`, Table Layout subsystem, HTML5 implicit `<tbody>`, DOM traversal (`matches`, `closest`, `contains`, `cloneNode`), Form APIs, and comprehensive test suites.
- [x] **v0.7.0**: Observers (`ResizeObserver`, `IntersectionObserver`, `MutationObserver`), Advanced Form Controls (`<select>`, `<option>`, `<textarea>`, radio group mutual exclusion), Storage foundation (`IndexedDB`, `CacheStorage`), `DOMParser` & `XMLSerializer`.
- [x] **v0.8.0**: Advanced HTTP cache & `Cache-Control`, ETag conditional requests, Content Security Policy (CSP), ServiceWorker registration, WebSocket & BroadcastChannel, Blob & File APIs.
- [x] **v0.9.0**: GPU Compositor layer tree (`LayerTree`, `DamageRegion`, `GpuCommand`), Indic/Assamese advanced HarfBuzz text shaping (অসমীয়া ligatures, BiDi RTL).
- [ ] **v1.0.0**: Process-isolated sandbox architecture, DevTools inspector protocol, production browser UI.

---

## 📜 License

MIT License © 2026 Samarjit Kashyap
