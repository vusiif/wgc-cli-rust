# rgc-cli (Rust Graphics Capture)

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)

Fast, lightweight, and cross-platform command-line screen capture utility written purely in Rust.

---

## 🌟 Features

- **Pure Cross-Platform**: Supports Windows, macOS, and Linux (X11 & Wayland) with zero OS-specific C/C++ glue code.
- **Modern Formats (PNG, JPEG, WebP)**: Native pure-Rust WebP encoding support, producing files ~30% smaller than PNG with pixel-perfect lossless clarity.
- **Structured JSON Mode**: `--json` flag formats both monitor lists and capture results as structured JSON, ideal for AI agents and script automation.
- **System Clipboard Export**: Direct-to-clipboard capture via `--clipboard` / `-c` (cross-platform via `arboard`) without touching the filesystem.
- **Stdout Streaming**: Stream raw encoded image bytes to `stdout` (`-o -`) for easy piping with `wl-copy`, `xclip`, ImageMagick, ffmpeg, or AI pipelines.
- **Smart Format Auto-Inference**: Automatically infers WebP, JPEG, and PNG from file extensions (e.g. `-o shot.webp`), with manual override via `--format`.
- **High-DPI Awareness**: Explicit translation of user logical coordinates into physical pixels with DPI scaling factor (`scale_factor`) for pixel-perfect cropping on Retina and 4K displays.
- **Multi-Monitor Support**: Inspect all connected displays and select any monitor by index (`--list-monitors`, `--monitor <INDEX>`).
- **Region Cropping**: Flexible rectangle cropping (`--region x,y,w,h`) using zero-copy sub-image operations in hot paths.
- **Delayed Capture**: Built-in timer delay before taking screenshot (`--delay <MS>`).

---

## 📦 Installation & Build

Ensure you have the Rust toolchain installed:

```bash
cargo build --release
```

The compiled release binary is located at `target/release/rgc-cli` (or `rgc-cli.exe` on Windows), weighing in at just **~2.2 MB**.

---

## 🚀 Usage Guide

```text
Usage: rgc-cli [OPTIONS]

Options:
  -l, --list-monitors         List all available monitors and exit
  -m, --monitor <INDEX>       Monitor index to capture (0-indexed). Defaults to primary monitor
  -r, --region <x,y,w,h>      Region to capture in logical coordinates: 'x,y,w,h' (e.g. 100,100,800,600)
  -o, --output <PATH>         Output file path, directory, or '-' for stdout stream. Defaults to timestamped file in CWD
  -c, --clipboard             Copy screenshot directly to system clipboard
  -d, --delay <MS>            Delay before capturing in milliseconds (e.g. 1000 for 1 second)
  -f, --format <FORMAT>       Output image format. If omitted, inferred from file extension [possible values: png, jpeg, webp]
      --json                  Output result as structured JSON (ideal for automation & AI agents)
  -v, --verbose               Enable verbose / debug logging
  -q, --quiet                 Suppress non-error output
  -h, --help                  Print help
  -V, --version               Print version
```

### Examples

#### 1. Copy directly to clipboard (no disk files created)
```bash
rgc-cli -c
```

#### 2. Pipe screenshot to another tool via stdout
```bash
# Save via shell redirection or downstream CLI
rgc-cli -o - > screenshot.png

# Linux Wayland / X11 clipboard pipe
rgc-cli -o - | wl-copy
```

#### 3. Save as modern WebP with structured JSON output
```bash
rgc-cli -o shot.webp --json
```
Output:
```json
{
  "status": "success",
  "path": "shot.webp",
  "width": 2560,
  "height": 1440,
  "format": "Webp",
  "clipboard": false,
  "size_bytes": 417652
}
```

#### 4. Query monitors in JSON format
```bash
rgc-cli --list-monitors --json
```
Output:
```json
[
  {
    "index": 0,
    "name": "\\\\.\\DISPLAY1",
    "is_primary": true,
    "width": 2560,
    "height": 1440,
    "scale_factor": 1.5
  }
]
```

#### 5. Dual output: Copy to clipboard AND save to file
```bash
rgc-cli -c -o ./archive/
```

#### 6. Delayed regional capture (HiDPI / Retina aware)
```bash
rgc-cli -d 1500 -r 200,200,600,400 -o cropped.png
```

---

## 🏛 Architecture

- [`src/main.rs`](src/main.rs): Clean CLI entry point, logger initialization, and top-level pipeline orchestration.
- [`src/cli.rs`](src/cli.rs): Type-safe command-line parsing powered by `clap` derive, including custom `Region` parser with boundary checks and serde derivations.
- [`src/capture.rs`](src/capture.rs): Core capture abstraction, DPI logical-to-physical coordinate mapping, multi-monitor querying, and zero-allocation cropping.
- [`src/output.rs`](src/output.rs): Clipboard export via `arboard`, stdout streaming, WebP/JPEG/PNG format inference, and idempotent filesystem operations.

---

## 🧪 Testing

Run all unit and integration tests:

```bash
cargo test
```
