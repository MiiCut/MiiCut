# MiiCut

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Status](https://img.shields.io/badge/status-WIP-orange.svg)
![Platform](https://img.shields.io/badge/platform-Web%20%7C%20WASM-lightgrey.svg)

**Work in progress — expect breaking changes and incomplete features.**

---

## Overview

**MiiCut** is an **open-source 2D CAD/CAM web application** focused on **cutting workflows** such as **laser, plasma, waterjet, and stencil cutting**.

It provides parametric shape drawing, SVG interoperability, robust boolean geometry operations, toolpath and G-code previews, and direct machine control for compatible CNC controllers.

MiiCut is designed to reliably generate **closed contours**, which are essential for CNC cutting operations.

---

## Key Features

- Parametric 2D drawing with editable vertices and live dimension handles
- Rotation handles per shape, snapping support
- SVG import/export
- Boolean operations (union / difference)
- Toolpath preview with correct hole-before-outer-contour ordering
- G-code preview
- Machine configuration and parameter management
- Direct machine control via grblHAL WebSocket (jog, home, zero, e-stop)
- Native support for **grblHAL-based machines**
- Embeddable on the machine itself — served directly from the grblHAL SD card

---

## Supported Use Cases

- Plasma cutting
- Laser cutting
- Waterjet cutting
- Drag knife and stencil cutting
- Rapid 2D CAM prototyping in the browser

---

## Typical Workflow

1. Draw or import geometry
2. Adjust parameters and boolean operations
3. Preview toolpaths
4. Inspect generated G-code
5. Configure machine parameters
6. Jog, home, zero, then execute the job

---

## Technology Stack

- **Rust** for core geometry, CAM, and application logic
- **WebAssembly (WASM)** compiled with `wasm-bindgen` and bundled by **Trunk**
- Zero JavaScript framework — pure Rust + `web-sys` DOM manipulation
- Runs fully client-side in a modern browser

---

## Compatibility

- CNC controllers: **grblHAL** (tested on RP2350 with networking plugin)
- Machine types: plasma, laser, waterjet, drag knife
- Browsers: modern Chromium / Firefox with WebAssembly support

> Always verify toolpaths and G-code before running on real machines.

---

## Project Status

- Active development
- APIs and UI subject to change
- Not yet production-ready

---

## Roadmap

- [ ] Additional parametric shapes
- [ ] Improved post-processors
- [ ] Advanced toolpath strategies
- [ ] Machine feedback and job monitoring
- [ ] Project save/load improvements
- [ ] Play / machine control view (jog interface, position display)

---

## Development

### Requirements

- Rust (stable)
- [Trunk](https://trunkrs.dev/)
- Python 3 (for deployment to grblHAL)

### Local development

```bash
trunk serve
```

Trunk is configured to always build in release mode (`Trunk.toml`).

### Deploy to grblHAL embedded server

The grblHAL networking plugin HTTP server only serves a single `index.html.gz` from the SD card `www/` folder. The deploy script bundles everything (WASM, JS, CSS, fonts, SVG icons) into that one file and uploads it via anonymous FTP.

```bash
trunk build
python3 deploy_to_grblhal.py            # bundle + upload
python3 deploy_to_grblhal.py --clean    # wipe www/ first, then upload
python3 deploy_to_grblhal.py --list     # inspect server contents
```

The target IP is configured at the top of `deploy_to_grblhal.py` (`FTP_HOST`).
