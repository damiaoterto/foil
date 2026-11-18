# Foil Core (`foil`)

**Foil** is an **AI-First**, native-performance presentation engine built in **Rust** (`tao` + `wry`) that renders HTML, CSS, and Vanilla JS/WebGL presentations (`1920x1080` at 60/120fps) inside a hermetic, 100% offline sandbox.

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
