# Charts (`chart.js`) & SVG Diagrams

## Contents
- `chart.js` Blueprint (Step Updates & Cleanup)
- Stage Styling Rules for `chart.js`
- Inline SVG Diagram Blueprint (`stroke-dashoffset` on Step)

## `chart.js` Blueprint (Step Updates & Cleanup)

Install `chart.js` safely: `install_package({ name: "chart.js" })` (or `bun add --ignore-scripts chart.js`).

Always give the `<canvas>` wrapper an explicit height, set `maintainAspectRatio: false`, drive dataset reveals in `onStep` via `chart.update()`, and call `chart.destroy()` in `onLeave(root)`.

```js
import Chart from "chart.js/auto";

let chart = null;

export default {
  steps: 2,
  html: `
    <section class="slide">
      <h2 class="h2-stage">Annual Capital Allocation (R$ bi)</h2>
      <div class="chart-wrap">
        <canvas data-chart></canvas>
      </div>
    </section>
  `,
  css: `
    .chart-wrap { position: relative; width: 100%; height: 620px; padding: 32px; border: 1px solid rgba(255,255,255,0.08); border-radius: 16px; background: #16181d; }
  `,
  onEnter(root) {
    const canvas = root.querySelector("[data-chart]");
    if (!canvas) return;

    chart = new Chart(canvas, {
      type: "bar",
      data: {
        labels: ["2023", "2024", "2025", "2026 (Oct)"],
        datasets: [
          {
            label: "Public CapEx (R$ bi)",
            data: [3.1, 3.8, 4.6, 0],
            backgroundColor: ["#334155", "#334155", "#334155", "#38bdf8"],
            borderRadius: 10
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { labels: { color: "#f4f4f5", font: { size: 18 } } }
        },
        scales: {
          x: { ticks: { color: "#9ca3af", font: { size: 18 } }, grid: { display: false } },
          y: {
            suggestedMax: 6,
            ticks: { color: "#9ca3af", font: { size: 18 } },
            grid: { color: "rgba(255,255,255,0.06)" }
          }
        }
      }
    });
  },
  onStep(_root, step) {
    if (!chart) return;
    if (step === 1) {
      chart.data.datasets[0].data[3] = 4.2;
      chart.update();
    }
    if (step === 2) {
      chart.data.datasets[0].data[3] = 5.4;
      chart.update();
    }
  },
  onLeave() {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  }
};
```

## Stage Styling Rules for `chart.js`

- **Readable stage fonts:** Set tick and legend font sizes to `16px–20px` (never `chart.js` default `12px`, which is illegible at `1920×1080`).
- **Low-noise grids:** Hide vertical grid lines (`grid: { display: false }`) and keep horizontal grid lines subtle (`rgba(255,255,255,0.06)` on dark, `rgba(0,0,0,0.06)` on light).
- **Tree-shaking option:** For smaller bundles, import `{ Chart, BarController, BarElement, CategoryScale, LinearScale, Tooltip, Legend }` from `"chart.js"` and call `Chart.register(...)` instead of `"chart.js/auto"`.

## Inline SVG Diagram Blueprint (`stroke-dashoffset` on Step)

For architecture flows or timelines without external dependencies, use inline `<svg>` (never `<script>` or `on*` inside SVG) and animate connectors via `stroke-dashoffset` in `onStep`:

```js
export default {
  steps: 1,
  html: `
    <section class="slide">
      <h2 class="h2-stage">Pipeline Flow</h2>
      <svg viewBox="0 0 1200 220" width="1200" height="220" fill="none">
        <rect x="20" y="60" width="260" height="100" rx="14" stroke="#38bdf8" stroke-width="2" />
        <rect x="920" y="60" width="260" height="100" rx="14" stroke="#34d399" stroke-width="2" />
        <path class="connector" d="M280 110 H920" stroke="#9ca3af" stroke-width="3" stroke-dasharray="640" stroke-dashoffset="640" />
      </svg>
    </section>
  `,
  css: `
    .connector { transition: stroke-dashoffset 0.6s cubic-bezier(0.16, 1, 0.3, 1); }
    .connector.drawn { stroke-dashoffset: 0; }
  `,
  onEnter(root) {
    root.querySelector(".connector")?.classList.remove("drawn");
  },
  onStep(root, step) {
    if (step === 1) root.querySelector(".connector")?.classList.add("drawn");
  },
  onLeave(root) {
    root.querySelector(".connector")?.classList.remove("drawn");
  }
};
```
