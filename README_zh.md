# rgc-cli (Rust Graphics Capture)

[![CI](https://github.com/vusiif/wgc-cli-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/vusiif/wgc-cli-rust/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/vusiif/wgc-cli-rust)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-brightgreen.svg)](https://modelcontextprotocol.io)

**[English](README.md)** | **[简体中文](README_zh.md)**

基于纯 Rust 构建的极速、轻量、跨平台屏幕截图命令行工具与 Model Context Protocol (MCP) 服务端。

> [!NOTE]
> `rgc-cli` 是原 `wgc-cli-rust` 项目的纯 Rust 跨平台演进版本。彻底移除对原生 C++ 胶水代码和本地构建工具链的依赖，在 Windows、macOS 和 Linux 上原生无缝运行。

---

## 🌟 核心特性

- **纯正跨平台**: 原生支持 Windows、macOS 以及 Linux (X11 与 Wayland)，采用 100% 内存安全代码（`#![forbid(unsafe_code)]`）。
- **Model Context Protocol (MCP)**: 内置 `--mcp` stdio 服务端，遵循官方 MCP JSON-RPC 2.0 规范，开箱即用支持 Claude Desktop、Cursor、Cline、Roo Code 等 AI 编程助手。
- **现代图片格式 (PNG, JPEG, WebP)**: 内置纯 Rust WebP 原生编码，相较传统 PNG 通常节省约 **20%~35%** 文件体积，且兼顾像素级清晰度。
- **结构化 JSON 输出**: `--json` 标志将显示器列表、截图元数据及错误回显为机器可读的 JSON 信封结构，专为脚本自动化与 Agent 设计。
- **系统自检诊断**: 内置 `--doctor` 标志，快速检测当前操作系统图形显示子系统、DPI 缩放、底层抓屏后端及系统剪贴板就绪状态。
- **系统剪贴板直写**: 支持 `--clipboard` / `-c` 直接将截图写入系统剪贴板（基于 `arboard` 跨平台支持），无需落盘。
- **标准输出流式管道**: 支持将编码图像直接流式输出至标准输出 (`-o -`)，优雅处理 `BrokenPipe`，可无缝对接 `wl-copy`、`xclip`、ImageMagick、ffmpeg 或 AI 图像处理流。
- **High-DPI 缩放感知**: 严密支持用户逻辑坐标系与显示物理像素之间的转换与 `scale_factor` 计算，在 4K 与 Retina 视网膜屏上精准对齐。
- **多显示器管理**: 列出并支持指定屏幕索引截图（`--list-monitors`, `--monitor <INDEX>`）。
- **矩形区域裁剪**: 支持 `--region x,y,w,h` 裁剪，并在全屏抓取前提供快速越界预校验，节省无谓内存与 GPU 开销。
- **原子防碰撞命名**: 自动生成的截屏文件名采用毫秒级时间戳搭配原子单调递增序列号，杜绝快速连拍下的文件覆盖。
- **延时捕获**: 支持毫秒级定时延时捕获（`--delay <MS>`）。

---

## 📦 安装与编译

确保已安装 Rust 工具链（推荐 1.75+）：

```bash
cargo build --release
```

编译产物位于 `target/release/rgc-cli`（Windows 下为 `rgc-cli.exe`），在 LTO 与符号裁剪（strip）优化下，二进制体积仅约 **2~3 MB**。

---

## 🚀 使用指南

```text
用法: rgc-cli [选项]

选项:
      --doctor                运行系统诊断环境检查，查看屏幕捕获与剪贴板就绪状态
      --mcp                   启动 Model Context Protocol (MCP) stdio 服务端供 AI 助手对接
  -l, --list-monitors         列出所有可用显示器并退出
  -m, --monitor <INDEX>       指定要捕获的显示器索引 (从 0 开始，默认为主显示器)
  -r, --region <x,y,w,h>      指定截取矩形区域坐标: 'x,y,w,h' (例如 '100,100,800,600')
      --coords <MODE>         区域坐标系: 'logical' (默认，按 DPI 缩放) 或 'physical' (原始物理像素)
  -o, --output <PATH>         输出文件路径、目录或 '-' (流式输出至 stdout)。默认为当前目录带时间戳文件
      --output-file <FILE>    显式指定输出文件目标 (若父目录不存在将自动创建)
      --output-dir <DIR>      显式指定输出目录目标 (在指定目录内生成防重时间戳文件)
  -c, --clipboard             将截图直接复制至系统剪贴板
  -d, --delay <MS>            截图前延时等待毫秒数 (例如 1000 表示延时 1 秒)
  -f, --format <FORMAT>       输出图片格式，缺省时自动根据扩展名推断 [可选: png, jpeg, webp]
      --scale <FACTOR>        按比例缩放图像 (0.01 到 1.0 之间，例如 0.5 缩小一半)
      --max-dimension <PX>    限制图像最大宽度或高度 (像素)，保持原始纵横比
      --quality <1-100>       JPEG 输出压缩质量 (1-100，默认 85)
      --open                  截图保存后自动调用系统默认查看器打开
      --completions <SHELL>   生成终端 Tab 自动补全脚本到 stdout [可选: bash, elvish, fish, powershell, zsh]
      --json                  以结构化 JSON 格式输出结果 (非常适合自动化与 AI 智能体)
  -v, --verbose               启用详细/调试日志
  -q, --quiet                 静默模式，抑制非错误信息
  -h, --help                  打印帮助信息
  -V, --version               打印版本信息
```

### 常用示例

#### 1. 直接复制到剪贴板（零磁盘写入）
```bash
rgc-cli -c
```

#### 2. 通过管道流式传输至其他工具
```bash
# 通过 Shell 重定向保存
rgc-cli -o - > screenshot.png

# Linux Wayland 剪贴板管道
rgc-cli -o - | wl-copy
```

#### 3. 保存为 WebP 并获取结构化 JSON 返回
```bash
rgc-cli -o shot.webp --json
```
输出：
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

#### 4. 存入指定目录（自动防碰撞时间戳命名）
```bash
rgc-cli --output-dir ./screenshots --json
```
输出：
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

#### 5. 以 JSON 格式查询所有显示器
```bash
rgc-cli --list-monitors --json
```
输出：
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

#### 6. 统一的结构化错误回显
在 `--json` 模式下发生错误时，将输出带退出码的机器可读诊断信息：
```bash
rgc-cli -m 999 --json
```
输出 (退出码 3)：
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

#### 7. 双重输出：保存文件的同时写入剪贴板
```bash
rgc-cli -c -o ./archive/
```

#### 8. 延时区域截图（感知 HiDPI 缩放）
```bash
rgc-cli -d 1500 -r 200,200,600,400 -o cropped.png
```

#### 9. 1:1 绝对物理像素裁剪
```bash
rgc-cli --coords physical -r 0,0,1920,1080 -o raw_pixels.png
```

#### 10. 图像尺寸缩放与质量控制（优化 AI Token 与传输带宽）
```bash
# 50% 等比下采样（例如 3840x2160 -> 1920x1080，体积缩减约 75%）
rgc-cli --scale 0.5 -o shot.webp

# 限制图片最长边不超过 1280px，保持原始纵横比
rgc-cli --max-dimension 1280 -o preview.png

# 指定 JPEG 压缩质量为 80
rgc-cli -f jpeg --quality 80 -o shot.jpg
```

#### 11. 截图后自动调用系统默认看图工具打开
```bash
# 截取并在系统默认照片/看图软件中即时打开
rgc-cli --open -o shot.png
```

#### 12. 生成终端 Tab 自动补全脚本
```bash
# 自动为 PowerShell 会话注入补全
rgc-cli --completions powershell | Out-String | Invoke-Expression

# 为 Linux/macOS Bash 或 Zsh 写入补全文件
rgc-cli --completions bash > /etc/bash_completion.d/rgc-cli
rgc-cli --completions zsh > "${fpath[1]}/_rgc-cli"
```

#### 10. 运行系统诊断检查
```bash
# 人类可读文本摘要
rgc-cli --doctor

# 结构化 JSON 诊断
rgc-cli --doctor --json
```
输出：
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

## 🤖 Model Context Protocol (MCP) 智能体集成

`rgc-cli` 内置标准 MCP stdio 服务端，支持 JSON-RPC 2.0 协议。可直接集成到 Claude Desktop、Cursor、Cline、Roo Code、Windsurf 等 AI 智能体中。

### 提供的 MCP 工具列表

1. **`list_monitors`**: 枚举所有屏幕、分辨率与 DPI 缩放系数。
2. **`capture_screen`**: 捕获指定屏幕或局部区域，支持直接返回 Base64 编码图片数据（`image/png`, `image/webp`, `image/jpeg`）或落盘至本地文件。
   - 参数：`monitor` (整数/字符串), `region` (字符串 `"x,y,w,h"`), `coords` (`"logical"` | `"physical"`), `format` (`"png"` | `"jpeg"` | `"webp"`), `outputPath` (字符串), `returnBase64` (布尔值)。
3. **`doctor`**: 诊断检查当前图形显示子系统与捕获就绪状态。

### 客户端接入配置示例

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

## 🚦 退出码与错误状态定义

| 退出码 | 错误码标识 | 阶段 (stage) | 说明 |
|:---:|:---|:---|:---|
| **0** | `SUCCESS` | `complete` | 操作成功执行 |
| **1** | `GENERIC_ERROR` | `execution` | 未分类的运行时错误 |
| **2** | `BAD_ARGS` | `cli_parsing` | 命令行参数格式不合法或互斥冲突 |
| **3** | `NO_MONITOR` / `MONITOR_NOT_FOUND` | `monitor_selection` | 未检测到显示器或指定的显示器索引超出范围 |
| **4** | `REGION_OUT_OF_BOUNDS` | `region_calculation` | 裁剪区域坐标超出了目标显示器边界 |
| **5** | `CAPTURE_FAILED` | `capture` | 操作系统捕获后端抓屏失败 |
| **6** | `FORMAT_MISMATCH` | `format_inference` | 显式 `--format` 格式与文件扩展名冲突 |
| **7** | `IO_ERROR` / `ENCODE_FAILED` | `output_save` | 磁盘写入失败、权限不足或编码失败 |
| **8** | `CLIPBOARD_ERROR` | `clipboard_export` | 系统剪贴板不可用或写入失败 |

---

## 📐 坐标系与 DPI 规范说明

- **坐标原点 `(0, 0)`**: 严格限定为目标显示器（默认为主显示器）的左上角。不支持负数虚拟桌面坐标混淆。
- **`--coords logical` (默认)**: 用户传入的数值视为逻辑坐标，将根据显示器 `scale_factor`（如 1.25x, 1.5x, 2.0x）进行防溢出四舍五入计算。
- **`--coords physical`**: 用户传入数值视为原始物理像素，1:1 精确映射，不进行任何 DPI 换算。
- **安全性保证**: 两种模式均会在调用昂贵的硬件截图前进行严格的边界预校验。

---

## 🏛 模块架构

- [`src/main.rs`](src/main.rs): 统一命令行入口、集中式错误分发与顶层流程管线。
- [`src/cli.rs`](src/cli.rs): 基于 `clap` derive 的类型安全参数解析，包含自定义 `Region` 解析与互斥冲突矩阵。
- [`src/capture.rs`](src/capture.rs): 跨平台底层抓屏抽象、DPI 逻辑/物理坐标映射、屏幕枚举与裁剪预校验。
- [`src/doctor.rs`](src/doctor.rs): 运行环境与硬件诊断、图形子系统就绪检测与报告构建。
- [`src/mcp.rs`](src/mcp.rs): MCP stdio 服务端协议实现、RFC 4648 高性能 Base64 编码与工具调用分发。
- [`src/output.rs`](src/output.rs): 跨平台剪贴板直写、stdout 流式管道、原子毫秒级防重命名、格式校验与 JSON Envelope 规范。

---

## 🧪 自动化测试

运行纯逻辑单元测试（适用于无桌面、无剪贴板的 CI 环境）：

```bash
cargo test
```

运行依赖真实桌面与剪贴板的硬件测试：

```bash
cargo test -- --ignored
```

---

## 📄 开源许可证

本项目基于 [Apache License, Version 2.0](LICENSE) 开源。
