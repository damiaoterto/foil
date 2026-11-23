---
name: foil-engine
description: >-
  Create, edit, preview, visually inspect, and package AI-first presentations
  with Foil Engine. Use when building slide decks, writing Vanilla JS slides
  (1920x1080), calling Foil MCP tools, or running `foil` CLI commands.
---

# Foil Engine

Foil is a native desktop (Rust `tao`/`wry`) and browser presentation engine for **1920×1080 (16:9)** Vanilla JS ES-Module decks packaged as offline, SHA-256-verified `.foil` archives.

## 1. Hard Rules (Security & Runtime)

Foil runs `.foil` packages inside a strict 6-layer sandbox (`connect-src 'none'`). Never violate these constraints:

- **Vanilla JS only:** Use pure ES Modules (`html`, `css?`, `onEnter`, `onStep`, `onLeave`). Do **not** use React, Vue, Svelte, or any Virtual DOM framework.
- **No dynamic code or Node built-ins:** Never use `eval(`, `new Function(`, `child_process`, `node:fs`, or `node:net`. Static analysis rejects them.
- **100% offline in production:** Production CSP sets `connect-src 'none'`. Bundle all fonts, images, scripts, and 3D assets locally—never rely on external CDNs, remote fonts, or `fetch()` to the internet.
- **Safe package installs:** Install npm packages only with `bun add --ignore-scripts` or the `install_package` MCP tool. Never use `npm`, `yarn`, or `pnpm`.
- **Clean up on exit:** Always clear `setTimeout`/`setInterval`, remove `window`/`document` listeners, and call `renderer.dispose()` (for Three.js/WebGL) inside `onLeave(root)`.
- **Asset allowlist & limits:** Only `.html`, `.js`, `.css`, `.json`, `.webp`, `.avif`, `.png`, `.jpg`, `.svg`, `.woff2`, `.mp4`, `.glb` are allowed. Keep packages `<= 1,000` files and `<= 250 MB`. Images `> 2560px` are auto-resized; `<script>`, `<foreignObject>`, and `on*` handlers are stripped from SVGs.

## 2. Project Structure & `manifest.json`

A Foil deck project uses Vite (`base: './'`) and LightningCSS:

```text
my-deck/
├── index.html
├── manifest.json
├── package.json          # uses @foil-engine/runtime
├── vite.config.ts
└── src/
    ├── main.js
    ├── styles/theme.css
    └── slides/
        ├── 01-cover.js
        └── 02-metrics.js
```

`manifest.json` schema:

```json
{
  "title": "Deck Title",
  "aspectRatio": "16:9",
  "width": 1920,
  "height": 1080,
  "theme": "minimal",
  "slides": [
    {
      "id": "01-cover",
      "file": "src/slides/01-cover.js",
      "steps": 0,
      "speakerNotes": "Optional presenter notes."
    }
  ]
}
```

## 3. Slide Module Contract (`src/slides/*.js`)

Each slide exports a default object rendered into a fixed `1920×1080` stage with **no scrollbars**:

```js
export default {
  steps: 2,
  html: `
    <section class="slide">
      <h2>Key Metrics</h2>
      <div class="card" data-step="1">120fps native DOM & WebGL</div>
      <div class="card" data-step="2">0 open ports offline</div>
    </section>
  `,
  css: `
    [data-step] { opacity: 0.25; transition: opacity 0.3s ease; }
    [data-step].step-visible { opacity: 1; }
  `,
  onEnter(root) {
    for (const el of root.querySelectorAll("[data-step]")) {
      el.classList.remove("step-visible");
    }
  },
  onStep(root, step) {
    const target = root.querySelector(`[data-step="${step}"]`);
    if (target) target.classList.add("step-visible");
  },
  onLeave(root) {
    for (const el of root.querySelectorAll("[data-step]")) {
      el.classList.remove("step-visible");
    }
  }
};
```

**Step lifecycle (0-based initial state):**
- Initial render (`step = 0`) mounts `html`/`css` and calls `onEnter(root)`. Do **not** depend on `onStep(root, 0)`.
- Advancing steps calls `onStep(root, 1)` through `onStep(root, steps)`.
- Leaving the slide calls `onLeave(root)`.

## 4. Slide Composition Discovery (Ask First)

When the user gives a broad prompt without specifying layout, depth, or visual direction, ask **3–5 concise clarifying questions** before generating slides:

1. **Audience & Narrative:** Who is the audience and what is the core takeaway?
2. **Slide Count & Density:** How many slides, and should they be headline-led or data/metric-heavy?
3. **Visual Direction:** Dark minimal or light editorial theme, and any brand/accent color?
4. **2D / 3D & Assets:** Which slides should use 3D `three` scenes, 2D `motion` choreography, SVG diagrams, or local images/logos?
5. **Data Sources:** Any specific metrics, documents, or URLs to ground the deck?

If the user already provided these details or asks to generate immediately, skip discovery and start authoring.

## 5. Workflows

### Option A: MCP Server Workflow (`foil mcp`)

Use the Foil MCP tools when authoring interactively with live visual feedback:

1. `create_deck({ title, theme?, aspect_ratio? })` — initializes `manifest.json`, `index.html`, and `src/slides/`.
2. `install_package({ name })` — installs a package safely via `bun add --ignore-scripts` (e.g., `three`).
3. `upsert_slide({ index, filename, code, speaker_notes? })` — writes `src/slides/<filename>` and registers it in `manifest.json`.
4. `inspect_slide({ slideIndex, stepIndex? })` — navigates to the slide/step and returns a base64 screenshot plus `diagnostics`.
5. `package_deck({ output_path })` — optimizes assets, generates SHA-256 `integrity.json`, and builds the `.foil` archive.

### Option B: CLI Workflow (`foil`)

- `foil create <name> --template <minimal|3d|pitch> [--no-install]` — scaffold a new deck.
- `foil init --template <minimal|3d|pitch> [--no-install]` — initialize in the current directory.
- `foil dev [--headless] [--browser] [--port 5173]` — start Vite + Foil Player with HMR and the dev bridge.
- `foil build [--target foil|web] [-o <path>]` — build a signed `.foil` archive (`foil`) or static web bundle (`web`).
- `foil open <file.foil> [--browser]` — open and present a `.foil` archive.

## 6. Visual Design, 2D Motion, Charts & 3D

For minimalist `1920×1080` visual polish, data visualization, and high-frame-rate animations, read these one-level references as needed:

- **Minimalist stage typography, safe areas & anti-AI-slop rules:** [references/design-and-typography.md](references/design-and-typography.md)
- **2D (`motion`) and 3D (`three`) blueprints with step choreography & cleanup:** [references/motion-and-3d.md](references/motion-and-3d.md)
- **Data charts (`chart.js`) and inline SVG diagrams:** [references/charts-and-diagrams.md](references/charts-and-diagrams.md)

Default library choices: use `motion` (`install_package({ name: "motion" })`) for 2D choreography, `chart.js` (`install_package({ name: "chart.js" })`) for data charts, and `three` (`install_package({ name: "three" })`) for 3D WebGL scenes.

## 7. Visual Verification Loop

After creating or updating any slide, verify it with `inspect_slide` (for step `0` and key `stepIndex` values):

1. Check `diagnostics.overflows` — if non-empty, elements exceed the `1920×1080` stage. Tighten typography, grid spacing, or padding until `overflows` is `[]`.
2. Check `diagnostics.consoleErrors` — fix any runtime exceptions or missing module imports.
3. Check `diagnostics.brokenAssets` — ensure all images, fonts, and `.glb` paths resolve locally.
4. Review the returned `screenshotBase64` for stage legibility (`>= 22px` body, `>= 52px` headings), contrast, whitespace, and alignment before packaging.
