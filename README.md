# FrameKey

A 2D animation application built with the [Nightshade](https://github.com/matthewjberger/nightshade) engine. Runs natively on desktop and in the browser via WebGPU.

<img width="2560" height="1392" alt="image" src="https://github.com/user-attachments/assets/62e8deff-56da-4ec4-b14c-a6c77445fac7" />

## Features

- **Drawing Tools**: Rectangle, Ellipse, Line, Pen (bezier curves), and Pencil (freehand drawing with Douglas-Peucker simplification)
- **Layer System**: Multiple layers with visibility, locking, and opacity controls
- **Keyframe Animation**: Place keyframes on a timeline with tweening (Linear, EaseIn, EaseOut, EaseInOut)
- **Object Properties**: Position, rotation, scale, fill color, stroke color, and stroke width — all animatable between keyframes
- **Selection and Transform**: Click to select objects, drag to move, Ctrl+click for multi-select
- **Onion Skinning**: Preview previous/next frames while editing
- **Undo/Redo**: Full project-level undo/redo history
- **Playback**: Real-time animation preview with configurable frame rate
- **Save/Load**: JSON-based `.anim` project files (native file dialogs on desktop, browser download/upload on WASM)
- **Export**: PNG sequence and sprite sheet export (native only)
- **Test Animation**: Built-in bouncing ball generator for quick testing (Insert > Generate Test Animation)

## Quickstart

```bash
# native
just run

# wasm (webgpu)
just run-wasm
```

> All chromium-based browsers like Brave, Vivaldi, Chrome, etc support WebGPU.
> Firefox also [supports WebGPU](https://mozillagfx.wordpress.com/2025/07/15/shipping-webgpu-on-windows-in-firefox-141/) now starting with version `141`.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `V` | Select tool |
| `R` | Rectangle tool |
| `E` | Ellipse tool |
| `L` | Line tool |
| `P` | Pen tool (bezier) |
| `B` | Pencil tool (freehand) |
| `O` | Toggle onion skinning |
| `Space` | Play / Pause |
| `F6` | Insert keyframe |
| `F7` | Insert blank keyframe |
| `Shift+F6` | Delete keyframe |
| `Left` | Previous frame |
| `Right` | Next frame |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` | Redo |
| `Ctrl+A` | Select all |
| `Delete` | Delete selected |
| `Ctrl+S` | Save (native only) |
| `Ctrl+Shift+S` | Save As (native only) |
| `Ctrl+O` | Open (native only) |

## Prerequisites

* [just](https://github.com/casey/just)
* [trunk](https://trunkrs.dev/) (for web builds)

> Run `just` with no arguments to list all commands

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.