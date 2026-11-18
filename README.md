# Foil Core (`foil`)

**Foil** is an **AI-First**, native-performance presentation engine built in **Rust** (`tao` + `wry`) that renders HTML, CSS, and Vanilla JS/WebGL presentations (`1920x1080` at 60/120fps) inside a hermetic, 100% offline sandbox.

---

## Why Not Just Write Plain HTML and Open It in a Browser?

This is the most important question about the project — and the short answer is: **plain HTML in a browser works for a quick prototype, but fails at distribution, stage control, and reliable AI authoring.**

If someone tries to present using only a loose `index.html` in Chrome, they immediately hit **5 real walls**:

### 1. The `file://` Protocol & Distribution Hell
* **Plain HTML:** Double-clicking a local `index.html` causes the browser to block ES Modules (`import`/`export`), `fetch()`, 3D model loading (`.glb`), and fonts due to CORS policies. To make it work, the recipient must unzip a folder full of files and spin up a local HTTP server (`npx serve`, Python, etc.) — completely impractical for sending a deck to a client or opening it on a conference laptop.
* **With Foil:** Everything compiles into a single `.foil` file, 100% offline (zero reliance on CDNs or venue Wi-Fi). Double-click and it opens directly via the native `foil://` protocol.

### 2. AI Generating Loose HTML Works "in the Dark"
* **Plain HTML:** When you ask an AI to generate an HTML/CSS presentation from scratch, the output routinely suffers from text overflowing the screen, unreadable font sizes on projectors, unwanted scrollbars, and inconsistent styles across slides.
* **With Foil:** The AI operates under a strict contract (fixed `1920x1080` logical canvas, semantic base layouts, and `onEnter`/`onStep`/`onLeave` lifecycle) and gets real visual feedback via MCP (`inspect_slide` returns a live screenshot + overflow diagnostics). The AI sees layout defects and fixes them before you even look.

### 3. A Web Browser Is Not a Stage Player
* **Plain HTML:** You fight address bars, tabs, browser extensions, system notifications, accidental trackpad pinch-zoom, and lack of native multi-monitor control. Having a **Presenter View** (speaker notes, timer, and next step on your laptop while the projector shows only the stage) requires fragile `window.open` hacks that popup blockers kill.
* **With Foil:** The Rust core (`tao` + `wry`) controls windows and multiple monitors natively, with zero browser chrome polluting the stage.

### 4. Memory & GPU Limits with Rich Animations (Three.js / Motion)
* **Plain HTML:** Putting multiple slides with Three.js scenes, Canvas, or heavy animations in a single DOM document keeps all WebGL contexts alive simultaneously — causing frame drops or hitting the browser's active WebGL context limit.
* **With Foil:** `@foil/runtime` isolates each slide as an independent module with explicit mounting and teardown (`onLeave` running `dispose()`), maintaining a rock-solid 60/120fps.

### 5. Automated Build Optimization
* **Plain HTML:** Either you depend on external links (CDNs that can fail right before you walk on stage), or you ship bloated images and unminified libraries without tree-shaking.
* **With Foil:** `foil build` tree-shakes JS/CSS, resizes and converts images to WebP/AVIF, verifies SHA-256 integrity, and packs everything into a ready-to-share `.foil` archive.

> **The 1-sentence answer:**
> *"HTML in the browser is the rendering engine, not the presentation format. Foil exists for the same reason `.pptx` is a packaged XML archive instead of opening loose XML files in Chrome: to provide single-file offline distribution, real stage control, and an environment where AI can actually see and validate what it builds."*

---

## Workspace Architecture (`core/`)

```text
core/
├── Cargo.toml
└── crates/
    ├── cli/          # `foil` binary (clap): create, init, dev, build, open, mcp
    ├── packager/     # Static scanner, SVG sanitizer, WebP optimizer, SHA-256 & .foil reader
    ├── player/       # Native window (tao) + WebView (wry) + foil:// protocol + Zero-Privilege IPC
    └── mcp-server/   # Model Context Protocol server (JSON-RPC 2.0 over stdio) for AI agents
```

---

## CLI Reference (`foil`)

| Command | Description |
| :--- | :--- |
| `foil create <name>` | Scaffold a new presentation project from the embedded template (`--template <minimal\|3d\|pitch>`, `--no-install`). |
| `foil init` | Initialize a presentation project in the current working directory. |
| `foil dev` | Start the Vite HMR dev server, launch the local inspection bridge, and open the native Player (`--headless`, `--browser`, `--port <5173>`). |
| `foil build` | Run static security checks, convert raster images to WebP, generate the SHA-256 `integrity.json` manifest, and package the `.foil` archive (`--target <foil\|web>`, `-o <path>`). |
| `foil open <file.foil>` | Verify SHA-256 integrity and open a `.foil` archive directly in the native WebView via the custom `foil://localhost/...` protocol without opening local HTTP ports. |
| `foil mcp` | Start the MCP server over `stdio`, exposing `create_deck`, `upsert_slide`, `install_package`, `inspect_slide`, and `package_deck`. |

### Native Player Keyboard Shortcuts (`foil open` / `foil dev`)

* **`→` / `Space` / `PageDown`:** Advance to the next step or slide.
* **`←` / `PageUp`:** Return to the previous step or slide.
* **`F11` / `F`:** Toggle borderless fullscreen mode.
* **`Esc`:** Exit fullscreen mode.

---

## Defense-in-Depth Security (6 Anti-Malware Layers)

Third-party `.foil` archives and templates are treated as untrusted inputs:

1. **WebView Sandbox & 100% Offline CSP:** Every `foil://localhost/...` response injects a strict `Content-Security-Policy` with `connect-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'`. External navigations, popups (`NewWindowResponse::Deny`), file downloads, and hardware permissions (`PermissionResponse::Deny`) are blocked at the engine level.
2. **Zero-Privilege IPC Shield:** The `window.ipc.postMessage` handler only accepts payloads matching the strict `SafeIpcMessage` enum (`PresenterSync`, `ToggleFullscreen`) with `#[serde(deny_unknown_fields)]`, exposing zero filesystem, shell, or OS access.
3. **Anti-Zip Bomb & Anti-Zip Slip:** Archives are limited to at most `1,000` files and `250 MB` uncompressed size, rejecting any path traversal (`..` or absolute paths) and enforcing a strict extension allowlist (`.html`, `.js`, `.css`, `.json`, `.webp`, `.avif`, `.png`, `.jpg`, `.svg`, `.woff2`, `.mp4`, `.glb`).
4. **SVG Sanitization & Static Code Scanner:** Strips `<script>`, `<foreignObject>`, `<iframe>`, inline `on*` event attributes, and `javascript:` URIs from SVGs; blocks builds containing `eval(`, `new Function(`, `child_process`, `node:fs`, or malicious `preinstall`/`postinstall` lifecycle scripts.
5. **Supply Chain Guard:** Dependency installation always enforces `bun install --ignore-scripts`.
6. **SHA-256 Cryptographic Integrity (`integrity.json`):** Every file inside a `.foil` archive is hashed with SHA-256 during `foil build` and cryptographically verified by `FoilArchiveReader` before execution.

---

## Development, Testing & Build

### Prerequisites
* **Rust** `1.85+` (Edition 2024)
* **Bun** `1.1+` (for project scaffolding and front-end bundling)
* **Linux (Ubuntu/Debian):** `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### Run Tests & Linter

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

### Build Release Binary

```bash
cargo build --release -p foil-cli
```

The compiled binary will be located at `target/release/foil`.

---

## Packaging & Releases (`.dmg`, `.deb`, `.msi`)

The GitHub Actions workflow [`.github/workflows/ci-release.yml`](./.github/workflows/ci-release.yml) runs the full test suite across Linux, macOS, and Windows and automatically builds and attaches native installers whenever a GitHub **Release** is created (or a `v*` tag is pushed):

* **macOS (`.dmg`):** Universal Binary (`aarch64-apple-darwin` + `x86_64-apple-darwin`) bundled inside `Foil.app` with `.foil` file association.
* **Linux (`.deb`):** Built via `cargo-deb` with `libwebkit2gtk-4.1-0` and `libgtk-3-0` runtime dependencies.
* **Windows (`.msi`):** Built via `cargo-wix` ([`crates/cli/wix/main.wxs`](./crates/cli/wix/main.wxs)) with system `PATH` registration and `.foil` file association.
