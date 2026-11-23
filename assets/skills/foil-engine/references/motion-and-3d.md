# High-Craft 2D (`motion`) & 3D (`three`) Blueprints

## Contents
- 2D Motion Blueprint (`motion` in Vanilla JS)
- 3D Studio Scene Blueprint (`three` with Step Camera & Full Cleanup)

## 2D Motion Blueprint (`motion` in Vanilla JS)

Install `motion` safely: `install_package({ name: "motion" })` (or `bun add --ignore-scripts motion`).

Animate **only** GPU-composited properties (`transform` / `x`, `y`, `scale`, `rotate`, and `opacity`). Always store returned controls and stop them in `onLeave(root)`.

```js
import { animate, stagger } from "motion";

let controls = [];

export default {
  steps: 2,
  html: `
    <section class="slide">
      <h2 data-enter>Zero-Port Offline Sandbox</h2>
      <div class="grid">
        <article class="tile" data-step="1"><h3>CSP Lockdown</h3><p>connect-src 'none'</p></article>
        <article class="tile" data-step="2"><h3>SHA-256</h3><p>Verified before load</p></article>
      </div>
    </section>
  `,
  css: `
    .slide [data-enter], .slide [data-step] { opacity: 0; transform: translateY(20px); }
    .tile { border: 1px solid rgba(255,255,255,0.08); border-radius: 14px; padding: 32px; }
  `,
  onEnter(root) {
    controls = [
      animate(
        root.querySelectorAll("[data-enter]"),
        { opacity: 1, y: 0 },
        { delay: stagger(0.08), duration: 0.5, ease: [0.16, 1, 0.3, 1] }
      )
    ];
  },
  onStep(root, step) {
    const target = root.querySelector(`[data-step="${step}"]`);
    if (!target) return;
    controls.push(
      animate(target, { opacity: 1, y: 0 }, { duration: 0.45, ease: [0.16, 1, 0.3, 1] })
    );
  },
  onLeave(root) {
    for (const ctrl of controls) ctrl.stop();
    controls = [];
    for (const el of root.querySelectorAll("[data-enter], [data-step]")) {
      el.style.opacity = "0";
      el.style.transform = "translateY(20px)";
    }
  }
};
```

## 3D Studio Scene Blueprint (`three` with Step Camera & Full Cleanup)

Install `three` safely: `install_package({ name: "three" })`.

Never ship bare primitives (lone cubes, spheres, or octahedrons) as a finished 3D object. Build **faithful, recognizable multi-part silhouettes** with real industrial/architectural detail:

- **Detailed Geometry:** Combine `ExtrudeGeometry` (beveled blades, chassis, fins), `LatheGeometry` (cylinders, vessels, domes), `TubeGeometry` along `CatmullRomCurve3` (cables, pipes, data buses), and `InstancedMesh` (LED arrays, server blades, particles).
- **Shadows & Grounding:** Enable `renderer.shadowMap.enabled = true` (`THREE.PCFSoftShadowMap`), cast/receive shadows on a matte plinth or floor, and add subtle floor rings or grid lines for scale.
- **Material Contrast:** Pair brushed metal (`metalness: 0.85, roughness: 0.22`), matte housing (`metalness: 0.15, roughness: 0.72`), clearcoat glass, and `emissive` status LEDs under `RoomEnvironment` (`PMREMGenerator`) + `ACESFilmicToneMapping`.
- **Living Motion:** Animate sub-assemblies (rotors, rings, oscillating pistons) and data/energy pulses traveling along `TubeGeometry` paths (`curve.getPointAt(t)`), while smoothly `lerp`-ing `camera.position` and `lookAt` on `onStep`.
- **Full Cleanup:** Dispose `envTarget`, every `geometry` and `material` via `scene.traverse`, `ResizeObserver`, `rafId`, and `renderer.dispose()` in `onLeave(root)`.

```js
import * as THREE from "three";

let rafId = 0;
let renderer = null;
let scene = null;
let resizeObserver = null;
let targetCamPos = new THREE.Vector3(0, 1.4, 4.2);

export default {
  steps: 2,
  html: `
    <section class="slide split">
      <div class="copy">
        <h2>3D Runtime</h2>
        <p data-step="1">Studio-lit PBR geometry</p>
        <p data-step="2">Step-driven camera interpolation</p>
      </div>
      <div class="stage-3d" data-canvas-host></div>
    </section>
  `,
  css: `
    .split { display: grid; grid-template-columns: 1fr 1.2fr; gap: 64px; height: 100%; align-items: center; }
    .stage-3d { width: 100%; height: 720px; border-radius: 16px; overflow: hidden; }
  `,
  onEnter(root) {
    const host = root.querySelector("[data-canvas-host]");
    if (!host) return;

    scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(40, host.clientWidth / host.clientHeight, 0.1, 100);
    targetCamPos.set(0, 1.4, 4.2);
    camera.position.copy(targetCamPos);

    renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "high-performance" });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(host.clientWidth, host.clientHeight);
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.1;
    host.appendChild(renderer.domElement);

    const hemi = new THREE.HemisphereLight(0xffffff, 0x1e293b, 0.9);
    const key = new THREE.DirectionalLight(0xffffff, 2.2);
    key.position.set(4, 6, 5);
    const rim = new THREE.DirectionalLight(0x93c5fd, 1.1);
    rim.position.set(-4, 2, -4);
    scene.add(hemi, key, rim);

    const mesh = new THREE.Mesh(
      new THREE.TorusKnotGeometry(0.9, 0.28, 160, 24),
      new THREE.MeshPhysicalMaterial({ color: 0xe4e4e7, roughness: 0.22, metalness: 0.65, clearcoat: 0.4 })
    );
    scene.add(mesh);

    resizeObserver = new ResizeObserver(() => {
      if (!renderer || !host.clientWidth || !host.clientHeight) return;
      camera.aspect = host.clientWidth / host.clientHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(host.clientWidth, host.clientHeight);
    });
    resizeObserver.observe(host);

    const tick = () => {
      rafId = requestAnimationFrame(tick);
      mesh.rotation.y += 0.006;
      camera.position.lerp(targetCamPos, 0.08);
      camera.lookAt(0, 0, 0);
      renderer.render(scene, camera);
    };
    tick();
  },
  onStep(_root, step) {
    if (step === 1) targetCamPos.set(2.2, 1.6, 3.4);
    if (step === 2) targetCamPos.set(-1.8, 0.9, 2.8);
  },
  onLeave() {
    cancelAnimationFrame(rafId);
    if (resizeObserver) resizeObserver.disconnect();
    resizeObserver = null;
    if (scene) {
      scene.traverse((obj) => {
        if (obj.geometry) obj.geometry.dispose();
        if (obj.material) {
          const mats = Array.isArray(obj.material) ? obj.material : [obj.material];
          for (const m of mats) m.dispose();
        }
      });
    }
    if (renderer) {
      renderer.dispose();
      renderer.domElement.remove();
    }
    renderer = null;
    scene = null;
  }
};
```
