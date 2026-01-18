# MiiCut

**Warning: this project is a work in progress. Expect changes and occasional
instability.**

Online version: https://miicut.github.io/MiiCut/

MiiCut is an all-in-one 2D CAD/CAM web app for cutting workflows (laser, plasma,
waterjet, stencil, etc.). You can draw dedicated shapes, import/export SVG,
preview the toolpath and G-code, and then send it to a compatible machine.

## Features

- Draw parametric 2D shapes with editable properties and vertices.
- Import/export SVG for interoperability with other tools.
- Toolpath preview and G-code preview tabs.
- Machine tab for configuration and monitoring.
- Direct control (jogging, configuration, run the generated G-code).
- Built for grblHAL-based machines (direct compatibility).

## Workflow

1. Draw or import shapes.
2. Adjust properties until the geometry is correct.
3. Preview the toolpath and G-code.
4. Configure the machine and run the job.

## Notes

- This is a Rust + WebAssembly application running in the browser.
- Always verify the toolpath and G-code before executing on a real machine.

## License

MIT
