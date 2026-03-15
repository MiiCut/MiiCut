<p align="center">
  <h1 align="center">✂️ MiiCut</h1>
  <p align="center">
    <strong>Open-source 2D CAD/CAM for CNC cutting machines</strong><br/>
    Draw · Boolean ops · Toolpath · G-code · Machine control
  </p>
  <p align="center">
    <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT"/>
    <img src="https://img.shields.io/badge/status-WIP-orange?style=flat-square" alt="WIP"/>
    <img src="https://img.shields.io/badge/Rust-WASM-B7410E?style=flat-square&logo=rust&logoColor=white" alt="Rust"/>
    <img src="https://img.shields.io/badge/grblHAL-RP2350-6e3b9c?style=flat-square" alt="grblHAL"/>
  </p>
</p>

---

## 🗺️ Overview

**MiiCut** is a browser-based 2D CAD/CAM application focused on **cutting workflows** — plasma, laser, waterjet, and drag knife.

It runs fully client-side (Rust + WebAssembly) and can be **served directly from the machine** via the grblHAL embedded HTTP server — no PC needed at the machine side.

---

## ✨ Key Features

- Parametric 2D drawing with editable vertices and live dimension handles
- Rotation handles per shape, angle snapping
- SVG import / export
- Boolean operations (union / difference)
- Toolpath preview — holes cut before outer contours
- G-code preview
- Machine parameters configuration
- Direct machine control via grblHAL WebSocket — jog, home, zero, e-stop
- **Embeds on the machine** — single `index.html.gz` served from grblHAL SD card

---

## 🔧 Supported Machines

| Type | Controller |
|---|---|
| Plasma cutting | grblHAL (tested on RP2350) |
| Laser cutting | grblHAL |
| Waterjet cutting | grblHAL |
| Drag knife / stencil | grblHAL |

> Always verify toolpaths and G-code before running on real machines.

---

## 🔄 Typical Workflow

1. Draw or import geometry
2. Apply boolean operations
3. Preview toolpaths
4. Inspect G-code
5. Configure machine parameters
6. Jog · Home · Zero · Cut

---

## 🧱 Technology Stack

- **Rust** — geometry, CAM logic, application state
- **WebAssembly** — compiled with `wasm-bindgen`, bundled by [Trunk](https://trunkrs.dev/)
- **No JS framework** — pure `web-sys` DOM manipulation
- Runs fully client-side in any modern browser

---

## ⬇️ Quick Install — grblHAL

No build required. Download the latest pre-built bundle from the [Releases](https://github.com/MiiCut/MiiCut/releases/latest) page and upload it to your machine.

### Requirements

- grblHAL controller with the **networking plugin** and an **SD card**
- FTP client (FileZilla, or any anonymous FTP)

### Steps

1. Download `index.html.gz` from the latest release
2. Connect to your machine via FTP (anonymous, no password):
   ```
   Host:  <your-machine-ip>
   Port:  21
   User:  anonymous
   Pass:  (leave blank)
   ```
3. Navigate to the `www/` folder (create it if it doesn't exist)
4. Upload `index.html.gz` into `www/`
5. Open `http://<your-machine-ip>/` in a browser

---

## 🚀 Development

### Requirements

- Rust (stable)
- [Trunk](https://trunkrs.dev/)
- Python 3 (deployment only)

### Local dev server

```bash
trunk serve
```

Trunk always builds in release mode (configured in `Trunk.toml`).

---

## 📦 Deploy to grblHAL

The grblHAL networking plugin HTTP server only serves a **single `index.html.gz`** from the SD card `www/` folder. The deploy script bundles everything — WASM, JS, CSS, fonts, SVG icons — into that one file and uploads it via anonymous FTP.

```bash
trunk build
python3 deploy_to_grblhal.py            # bundle + upload to machine
python3 deploy_to_grblhal.py --clean    # wipe www/ first, then upload
python3 deploy_to_grblhal.py --bundle   # bundle only → index.html.gz (no upload)
python3 deploy_to_grblhal.py --list     # inspect server contents
```

The machine IP is configured at the top of `deploy_to_grblhal.py` (`FTP_HOST`).

---

## 🗺️ Roadmap

- [ ] Additional parametric shapes
- [ ] Improved post-processors
- [ ] Advanced toolpath strategies
- [ ] Machine feedback and job monitoring
- [ ] Project save / load
- [ ] Play / machine control view (jog interface, position display)

---

<p align="center">
  <sub>Built with 🧡 by Olivier (Mool)</sub>
</p>
