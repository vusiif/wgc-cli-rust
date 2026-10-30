# rgc-cli (Rust Graphics Capture)

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/vusiif/wgc-cli-rust)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-brightgreen.svg)](https://modelcontextprotocol.io)

**[English](README.md)** | **[简体中文](README_zh.md)**

Fast, lightweight, and cross-platform command-line screen capture utility and Model Context Protocol (MCP) server written purely in Rust.

> [!NOTE]
> `rgc-cli` is the cross-platform, pure-Rust evolution of the original `wgc-cli-rust` project. It eliminates all platform-dependent C++ glue and runs natively on Windows, macOS, and Linux with zero external native build dependencies.

---

## 🌟 Features

- **Pure Cross-Platform**: Native support for Windows, macOS, and Linux (X11 & Wayland) with 100% safe Rust code (`#![forbid(unsafe_code)]`).
- **Model Context Protocol (MCP)**: Built-in `--mcp` stdio server implementing the official MCP JSON-RPC 2.0 specification for seamless AI agent integration (Claude Desktop, Cursor, Cline, Roo Code).
- **Modern Formats (PNG, JPEG, WebP)**: Native WebP encoding support, typically delivering ~20-35% file size reduction compared to PNG with lossless or near-lossless clarity.
- **Structured JSON Mode**: `--json` flag formats monitor lists, captured metadata, and errors into machine-readable JSON envelopes, ideal for scripting and automation.
- **System Diagnostics**: Built-in `--doctor` flag to inspect platform display subsystems, DPI scales, capture backends, and clipboard readiness.
- **Direct Clipboard Export**: Copy screenshots directly to the system clipboard via `--clipboard` / `-c` without touching the filesystem.
- **Stdout Streaming**: Stream raw encoded image bytes to stdout (`-o -`) with graceful `BrokenPipe` handling for seamless UNIX piping (`wl-copy`, `xclip`, ImageMagick, ffmpeg).
- **High-DPI Awareness**: Accurate translation between user logical coordinates and physical screen pixels with display `scale_factor` (e.g. 1.25x, 1.5x, 2.0x).
- **Multi-Monitor Selection**: Enumerate all connected displays and capture any screen by index (`--list-monitors`, `--monitor <INDEX>`).
- **Sub-Image Region Cropping**: Flexible rectangle cropping (`--region x,y,w,h`) with fast bounds pre-validation.
- **Collision-Resistant Naming**: Millisecond-accurate timestamping with atomic monotonic counter prevents filename collisions in rapid capture loops.
- **Delayed Capture**: Built-in timer delay before taking screenshot (`--delay <MS>`).

---

## 📦 Installation & Build

Ensure you have the Rust toolchain installed (1.75+ recommended):

```bash
cargo build --release
```

The compiled release binary is located at `target/release/rgc-cli` (or `rgc-cli.exe` on Windows), typically weighing in at approximately **~2 to 3 MB** thanks to LTO and binary stripping.

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
# Save via shell redirection
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

#### 6. Structured Error Response
When an error occurs in `--json` mode, machine-readable diagnostic details are returned to stdout with an appropriate non-zero exit code:
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
# Human-readable text report
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

---

## 🤖 Model Context Protocol (MCP) Integration

`rgc-cli` natively acts as an MCP stdio server conforming to JSON-RPC 2.0. This allows AI coding agents (Claude Desktop, Cursor, Cline, Roo Code, Windsurf) to directly take screenshots and inspect displays.

### Exposed MCP Tools

1. **`list_monitors`**: Enumerate all connected screens, resolutions, and DPI scale factors.
2. **`capture_screen`**: Capture screen or cropped region with direct base64 image payload (`image/png`, `image/webp`, `image/jpeg`) or output file.
   - Arguments: `monitor` (int/str), `region` (string `"x,y,w,h"`), `coords` (`"logical"` | `"physical"`), `format` (`"png"` | `"jpeg"` | `"webp"`), `outputPath` (string), `returnBase64` (bool).
3. **`doctor`**: Diagnostic check of display subsystem, active backend, and clipboard capabilities.

### Client Configuration Examples

#### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "screen-capture": {
      "command": "rgc-cli",
      "args": ["--mcp"]
    }
  }
}
```

#### Cline / Roo Code / Cursor (`mcp_settings.json`)

```json
{
  "mcpServers": {
    "rgc": {
      "command": "/absolute/path/to/rgc-cli",
      "args": ["--mcp"]
    }
  }
}
```

---

## 🚦 Exit Codes & Error Specification

In `--json` mode or standard CLI usage, exit codes adhere strictly to the following specification:

| Exit Code | Error Code Identifier | Stage | Description |
|:---:|:---|:---|:---|
| **0** | `SUCCESS` | `complete` | Operation succeeded |
| **1** | `GENERIC_ERROR` | `execution` | Uncategorized failure |
| **2** | `BAD_ARGS` | `cli_parsing` | Invalid arguments or conflicting CLI flags |
| **3** | `NO_MONITOR` / `MONITOR_NOT_FOUND` | `monitor_selection` | No display found or specified index is out of bounds |
| **4** | `REGION_OUT_OF_BOUNDS` | `region_calculation` | Crop coordinates exceed target monitor dimensions |
| **5** | `CAPTURE_FAILED` | `capture` | OS capture API failed to capture display buffer |
| **6** | `FORMAT_MISMATCH` | `format_inference` | Explicit `--format` contradicts file extension |
| **7** | `IO_ERROR` / `ENCODE_FAILED` | `output_save` | Disk write error, missing permissions, or encoding failure |
| **8** | `CLIPBOARD_ERROR` | `clipboard_export` | System clipboard unavailable or write failed |

---

## 📐 Coordinate System & DPI Semantics

- **Origin `(0, 0)`**: Strictly defined as the top-left corner of the selected monitor (or primary monitor by default).
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

---

## 📄 License

Licensed under the [Apache License, Version 2.0](LICENSE).
