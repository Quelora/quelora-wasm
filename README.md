# quelora-wasm

**WebAssembly modules for the Quelora widget.**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](./LICENSE)

Rust crates compiled to WebAssembly, used inside the Quelora widget's Web
Worker to keep heavy work off the main thread.

## Modules

| Module | Purpose |
|--------|---------|
| **Image processor** | Resize and compress images before upload |
| **Markdown parser** | Convert user Markdown to sanitized, XSS-safe HTML |

## Build

Built with [`wasm-pack`](https://rustwasm.github.io/wasm-pack/). See the build
script in this repository.

```bash
./build.sh
```

The generated `.wasm` files and JS bindings are consumed by
[`quelora-widget-community`](https://github.com/Quelora/quelora-widget-community).

## Requirements

- Rust toolchain (stable) · `wasm-pack`

## License

[AGPL-3.0-only](./LICENSE) — Copyright (C) 2026 Germán Zelaya.

Part of the **[Quelora](https://github.com/Quelora)** project.
