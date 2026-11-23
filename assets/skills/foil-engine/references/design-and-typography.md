# Impeccable 1920×1080 Stage Design & Typography

## Contents
- Stage Geometry & Safe Area
- Typography Scale & Craft
- Color, Tint & Contrast
- Anti-AI-Slop Bans & Layout Variety

## Stage Geometry & Safe Area

- **Fixed logical canvas:** `1920×1080` (`16:9`), `overflow: hidden` — never design scrolling slides.
- **Required `#foil-root` centering:** `FoilDeck` scales `.foil-stage` from `center center`. Always style `#foil-root` as a centered flex container or the scaled stage will clip off-screen:
  ```css
  #foil-root {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    position: relative;
  }
  ```
- **Outer safe-area padding:** `88px–96px` top/bottom, `104px–120px` left/right. Leave 35–40% open whitespace so the focal artifact leads.

## Typography Scale & Craft

Web body sizes (`14px–16px`) are illegible on `1920×1080`. Use stage-scale proportions with strong weight/size jumps and tight display tracking:

| Role | Size | Weight | Line Height | Tracking |
|---|---|---|---|---|
| Hero / Cover Title | `84px–112px` | `700–800` | `1.02` | `-0.035em` |
| Slide Heading | `52px–68px` | `700` | `1.08` | `-0.025em` |
| Lead / Subtitle | `28px–34px` | `400–500` | `1.35` | `-0.01em` |
| Body Copy | `22px–26px` | `400` | `1.45` | `0` |
| Big Stat / Data | `64px–88px` | `700` | `1.0` | `-0.03em` |

- **Tabular numerals:** Always set `font-variant-numeric: tabular-nums` on metrics, tables, and counters.
- **Offline fonts:** Bundle `.woff2` fonts locally in `src/assets/fonts/` via `@font-face` (`connect-src 'none'` blocks external font CDNs).
- **Spacing:** Always put more vertical space above a heading than below it so headings bind to their content.

## Color, Tint & Contrast

- **Committed palette:** Pick light or dark from the presentation's ambient room, then commit to 1 canvas, 1 surface, and 1–2 purposeful accents.
- **Tint secondary text:** On colored surfaces, tint secondary text from that hue or the foreground—never dull gray over saturated fills. Maintain `>= 4.5:1` contrast for body copy and `>= 3:1` for large headings.
- **Clean borders:** Use full `1px` hairline borders (`rgba(255,255,255,0.09)` or `#e5e3dc`) rather than one-sided accent bars on rounded boxes.

## Anti-AI-Slop Bans & Layout Variety

Inspired by `pbakaus/impeccable`, refuse generic AI slide scaffolds:

- **No eyebrow / kicker labels above headings:** Let the headline carry its own weight without an uppercase tag above every title.
- **No uniform card grids:** Avoid repeating identical "3 equal cards with icon + title + text" across slides. Vary layouts: asymmetric splits, full-bleed SVG/3D diagrams, typographic spreads, and borderless hairline tables.
- **No nested cards:** Never place cards inside cards.
- **No decorative crutches:** Ban gradient text, heavy drop shadows, neon halos, and emojis as icons (author crisp inline `<svg>` with consistent stroke width).
- **One authored motion moment:** Instead of repeating the exact same `fade-up` stagger on every slide, craft one purposeful motion per slide (e.g., `clip-path` reveal, `stroke-dashoffset` diagram draw, or 3D camera move).
