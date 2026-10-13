# Release Notes - v2.0.0: The Cross-Platform Rewrite (`rgc-cli`)

This major release marks the complete transformation of the legacy Windows-only `wgc-cli` into **`rgc-cli` (Rust Graphics Capture)** — a purely cross-platform, high-performance, and lightweight CLI utility written 100% in Rust.

---

## 🚀 Key Highlights & New Features

### 1. Pure Cross-Platform Architecture
- **Zero OS-specific C/C++ bindings**: Completely eliminated the MSVC C++ WGC glue layer (`wgc_capture.cpp`) and native Windows dependencies.
- **Full Tier-1 Platform Support**: Works seamlessly on **Windows**, **macOS**, and **Linux** (both X11 and Wayland sessions) powered by `xcap`.

### 2. Modern Formats: Lossless Pure-Rust WebP
- Added first-class support for **WebP** (`--format webp`, or auto-inferred via `-o output.webp`), reducing screenshot file size by **~30%** compared to PNG while retaining lossless, pixel-perfect visual clarity.
- Continued full support for standard **PNG** and **JPEG** encoders.

### 3. Direct-to-Clipboard Export (`--clipboard` / `-c`)
- Directly copy captured frames to the system clipboard across all platforms without generating temporary files on disk.
- Supports dual export: copy to clipboard and save to disk simultaneously (`-c -o ./path`).

### 4. Unix Stdout Streaming (`-o -`)
- Stream raw encoded image bytes directly to standard output, enabling seamless shell piping into tools like `wl-copy`, `xclip`, `ImageMagick`, `ffmpeg`, or AI agent pipelines.

### 5. High-DPI & Retina Scaling Awareness
- Explicit translation between logical user coordinates and physical pixel buffers using monitor `scale_factor`.
- Region cropping (`--region x,y,w,h`) maps precisely across 100%, 125%, 150%, and 200%+ display scalings with zero-cost sub-image slicing in hot paths.

### 6. Structured JSON Output (`--json`)
- `--list-monitors --json`: Outputs monitor indices, IDs, physical resolutions, and DPI factors as a structured JSON array.
- `--json` on capture: Emits a structured JSON report including saved path, dimensions, format, file size, and clipboard status.

### 7. Multi-Monitor Selection & Delayed Capture
- `--list-monitors` (`-l`): Human-readable or JSON display enumeration.
- `--monitor` (`-m`): Capture a specific display by index (defaults to primary monitor).
- `--delay` (`-d`): Millisecond-level countdown timer before capture.

---

## 📊 Performance & Binary Size
- **Standalone Release Binary**: ~2.2 MB (fully optimized with LTO and symbol stripping).
- **Fast Startup & Clean Exit**: Zero background daemon overhead, zero memory leaks.

---

## 🛠 Breaking Changes & Migration from v1.x
- Binary name renamed from `wgccli.exe` to `rgc-cli`.
- Removed Windows-specific named pipe IPC server/client mode and D3D diagnostics in favor of standard cross-platform command-line execution and stdout streaming.
