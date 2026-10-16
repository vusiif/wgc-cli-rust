# rgc-cli (Rust Graphics Capture)

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)

Fast, lightweight, and cross-platform command-line screen capture utility written purely in Rust.

---

## 🌟 Features

- **Pure Cross-Platform**: Supports Windows, macOS, and Linux (X11 & Wayland) with zero OS-specific C/C++ glue code.
- **Modern Formats (PNG, JPEG, WebP)**: Native pure-Rust WebP encoding support, typically producing significantly smaller files than PNG (often ~20-35% savings depending on image entropy) with pixel-perfect clarity.
- **Structured JSON Mode**: `--json` flag formats both monitor lists and capture results as structured JSON, ideal for AI agents and script automation.
- **Model Context Protocol (MCP)**: Built-in `--mcp` stdio server implementing the official MCP specification with tools for screen capture (base64 image return), monitor enumeration, and system diagnostics.
- **System Diagnostics**: Built-in `--doctor` flag to inspect platform display subsystems, DPI scales, capture backends, and clipboard readiness.
- **System Clipboard Export**: Direct-to-clipboard capture via `--clipboard` / `-c` (cross-platform via `arboard`) without touching the filesystem.
- **Stdout Streaming**: Stream raw encoded image bytes to `stdout` (`-o -`) with graceful `BrokenPipe` handling for seamless piping with `wl-copy`, `xclip`, ImageMagick, ffmpeg, or AI pipelines.
- **Smart Format Auto-Inference**: Automatically infers WebP, JPEG, and PNG from file extensions (e.g. `-o shot.webp`), with manual override via `--format`.
- **High-DPI Awareness**: Explicit translation of user logical coordinates into physical pixels with DPI scaling factor (`scale_factor`) for pixel-perfect cropping on Retina and 4K displays.
- **Multi-Monitor Support**: Inspect all connected displays and select any monitor by index (`--list-monitors`, `--monitor <INDEX>`).
- **Region Cropping**: Flexible rectangle cropping (`--region x,y,w,h`) using zero-copy sub-image operations on the captured frame buffer.
- **Delayed Capture**: Built-in timer delay before taking screenshot (`--delay <MS>`).

---

## 📦 Installation & Build

Ensure you have the Rust toolchain installed:

```bash
cargo build --release
```

The compiled release binary is located at `target/release/rgc-cli` (or `rgc-cli.exe` on Windows), typically weighing in at approximately **~2 to 3 MB** depending on target platform and LTO settings.

---

## 🚀 Usage Guide

```text
Usage: rgc-cli [OPTIONS]

Options:
      --doctor                Run diagnostic environment check and inspect capture capabilities
      --mcp                   Start Model Context Protocol (MCP) stdio server for AI agent automation
  -l, --list-monitors         List all available monitors and exit
  -m, --monitor <INDEX>       Monitor index to capture (0-indexed). Defaults to primary monitor
  -r, --region <x,y,w,h>      Region to capture in coordinates: 'x,y,w,h' (e.g. 100,100,800,600)
      --coords <MODE>         Coordinate system for region: 'logical' (default, scaled by DPI) or 'physical' (raw pixels)
  -o, --output <PATH>         Output file path, directory, or '-' for stdout stream. Defaults to timestamped file in CWD
      --output-file <FILE>    Explicit output file destination (creates parent directories if missing)
      --output-dir <DIR>      Explicit output directory destination (timestamped filename generated inside)
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
  "ok": true,
  "data": {
    "status": "success",
    "path": "shot.webp",
    "width": 2560,
    "height": 1440,
    "format": "Webp",
    "clipboard": false,
    "sizeBytes": 417652
  }
}
```

#### 4. Save into a directory with auto-generated collision-free timestamp
```bash
rgc-cli --output-dir ./screenshots --json
```
Output:
```json
{
  "ok": true,
  "data": {
    "status": "success",
    "path": "screenshots/screenshot_2026-10-16_00-00-19.163_0000.png",
    "width": 2560,
    "height": 1440,
    "format": "Png",
    "clipboard": false,
    "sizeBytes": 574834
  }
}
```

#### 5. Query monitors in JSON format
```bash
rgc-cli --list-monitors --json
```
Output:
```json
{
  "ok": true,
  "data": [
    {
      "index": 0,
      "name": "\\\\.\\DISPLAY1",
      "is_primary": true,
      "width": 2560,
      "height": 1440,
      "scale_factor": 1.5
    }
  ]
}
```

#### 6. Structured Error Response Example
When an error occurs in `--json` mode, machine-readable diagnostic details are returned to stdout with an appropriate exit code:
```bash
rgc-cli -m 999 --json
```
Output (Exit Code 3):
```json
{
  "ok": false,
  "error": {
    "errorCode": "MONITOR_NOT_FOUND",
    "message": "Monitor index 999 is out of bounds (found 1 monitors).",
    "stage": "monitor_selection",
    "suggestion": "Run 'rgc-cli --list-monitors' to see all valid monitor indices.",
    "exitCode": 3
  }
}
```

#### 7. Dual output: Copy to clipboard AND save to file
```bash
rgc-cli -c -o ./archive/
```

#### 8. Delayed regional capture (HiDPI / Retina aware)
```bash
rgc-cli -d 1500 -r 200,200,600,400 -o cropped.png
```

#### 9. Exact 1:1 physical pixel crop
```bash
rgc-cli --coords physical -r 0,0,1920,1080 -o raw_pixels.png
```

#### 10. Run system diagnostics
```bash
# Text report
rgc-cli --doctor

# Structured JSON report
rgc-cli --doctor --json
```
Output:
```json
{
  "ok": true,
  "data": {
    "os": "windows",
    "arch": "x86_64",
    "backend": "Windows Graphics Capture (WGC) / DXGI / GDI",
    "monitorsDetected": 1,
    "primaryMonitor": "[0] \\\\.\\DISPLAY1 - 2560x1440 @ 1.50x",
    "clipboardAvailable": true,
    "supportedFormats": [
      "PNG (Lossless)",
      "JPEG (Standard)",
      "WebP (High-efficiency native)"
    ],
    "status": "Operational"
  }
}
```

#### 11. Run Model Context Protocol (MCP) Server for AI Agents
`rgc-cli` natively acts as an MCP server over stdio using standard JSON-RPC 2.0 (compatible with Claude Desktop, Cursor, Cline, Roo Code, etc.):
```bash
rgc-cli --mcp
```

**Exposed MCP Tools:**
- `list_monitors`: Enumerate connected displays with resolution and DPI scale factor.
- `capture_screen`: Capture display or specified region with direct base64 image return (`image/png`, `image/webp`, `image/jpeg`) or file destination.
- `doctor`: Inspect active display subsystem and screen capture readiness.

---

## 📐 Coordinate System & DPI Semantics

- **Origin `(0, 0)`**: Strictly defined as the top-left corner of the selected monitor (or primary monitor by default). Regions do not operate in arbitrary negative virtual desktop spaces.
- **`--coords logical` (Default)**: User-provided values are treated as logical screen coordinates and multiplied by the display's DPI `scale_factor` (e.g. 1.25x, 1.5x, 2.0x) using overflow-safe rounded arithmetic.
- **`--coords physical`**: User-provided values represent raw display buffer pixels directly, 1:1 without scaling.
- **Safety**: Both modes strictly validate coordinate boundaries, non-zero positive dimensions, and verify against the actual captured frame buffer dimensions.

---

## 🏛 Architecture

- [`src/main.rs`](src/main.rs): Clean CLI entry point, centralized error handling, and top-level pipeline orchestration.
- [`src/cli.rs`](src/cli.rs): Type-safe command-line parsing powered by `clap` derive, including custom `Region` parser with boundary checks and serde derivations.
- [`src/capture.rs`](src/capture.rs): Core capture abstraction, DPI logical-to-physical coordinate mapping, multi-monitor querying, and bounds checking.
- [`src/doctor.rs`](src/doctor.rs): Environment and hardware diagnostics, subsystem capability checks, and structured reporting.
- [`src/mcp.rs`](src/mcp.rs): Model Context Protocol (MCP) stdio server implementation, RFC 4648 base64 encoding, and tool execution.
- [`src/output.rs`](src/output.rs): Clipboard export via `arboard`, stdout streaming with BrokenPipe safety, collision-resistant timestamping, format validation, and unified JSON envelope schema.

---

## 🧪 Testing

Run all unit tests in headless environments (no physical display or clipboard needed):

```bash
cargo test
```

Run hardware-dependent tests (requires active graphical desktop session and clipboard access):

```bash
cargo test -- --ignored
```
