# MiiCut

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Status](https://img.shields.io/badge/status-WIP-orange.svg)
![Platform](https://img.shields.io/badge/platform-Web%20%7C%20WASM-lightgrey.svg)

**⚠️ Work in progress — expect breaking changes and incomplete features.**

🌐 Online demo: https://miicut.github.io/MiiCut/

---

## Overview

**MiiCut** is an **open-source 2D CAD/CAM web application** focused on **cutting workflows** such as **laser, plasma, waterjet, and stencil cutting**.

It provides parametric shape drawing, SVG interoperability, robust boolean geometry operations, toolpath and G-code previews, and direct machine control for compatible CNC controllers.

MiiCut is designed to reliably generate **closed contours**, which are essential for CNC cutting operations.

---

## Key Features

- Parametric 2D drawing with editable properties and vertices
- SVG import/export
- Boolean operations (union / difference)
- Toolpath preview
- G-code preview
- Machine configuration and monitoring
- Direct machine control (jogging, run jobs)
- Native support for **grblHAL-based machines**

---

## Supported Use Cases

- Laser cutting
- Plasma cutting
- Waterjet cutting
- Drag knife and stencil cutting
- Rapid 2D CAM prototyping in the browser

---

## Typical Workflow

1. Draw or import geometry
2. Adjust parameters and boolean operations
3. Preview toolpaths
4. Inspect generated G-code
5. Configure the machine
6. Execute the job

---

## Technology Stack

- **Rust** for core geometry, CAM, and logic
- **WebAssembly (WASM)** for browser execution

Runs fully client-side in a modern browser.

---

## Compatibility

- CNC controllers: **grblHAL**
- Machine types: laser, plasma, waterjet, drag knife
- Browsers: modern Chromium / Firefox with WebAssembly support

> ⚠️ Always verify toolpaths and G-code before running on real machines.

---

## Project Status

- 🚧 Active development
- 🔄 APIs and UI subject to change
- ❗ Not yet production-ready
- 🧪 Testing coverage in progress

---

## Roadmap

- [ ] Additional parametric shapes
- [ ] Improved post-processors
- [ ] Advanced toolpath strategies
- [ ] Machine feedback and job monitoring
- [ ] Project save/load improvements

---

## Development

The application is built and served using a Rust + Trunk workflow.

### Requirements

- Rust (stable)
- Trunk

### Build

```bash
cargo b --release
trunk serve --release
```
