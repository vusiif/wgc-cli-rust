# wgc-cli-rust 项目状态与环境总结报告

- **报告生成时间**: 2026-10-13
- **项目路径**: `C:\Users\IceLolly\Music\Rust\wgc-cli-rust`
- **当前版本**: v1.1.0
- **Git 分支**: `rust-rewrite` (与远程分支保持同步)

---

## 一、 开发环境与工具链状态

| 模块 / 工具 | 版本 / 状态 | 说明 |
| :--- | :--- | :--- |
| **IDE** | JetBrains RustRover | 已集成并加载 `wgc-cli-rust` 模块，Run Configuration (`Run`) 正常就绪 |
| **Rust 编译器** | `rustc 1.96.0` | Target: `x86_64-pc-windows-msvc` |
| **包管理器** | `cargo 1.96.0` | `Cargo.lock` 已锁定稳定依赖版本 |
| **C/C++ 工具链** | MSVC (`cl.exe`) | 用于编译 `build.rs` 引用的 WGC 底层 C++ 胶水代码 (`src/wgc_capture.cpp`) |
| **构建与检查状态** | **PASS** | `cargo check`、`cargo test` 与 RustRover IDE `build_project` 均通过，0 错误 |

---

## 二、 项目定位与核心特性

本项目为 Windows Graphics Capture (WGC) 命令行工具从 C++ 到 Rust 的高性能重构版本，已达成功能对齐（Feature Parity）并进行了体系化增强：

### 1. 核心捕获与图形集成
- **技术栈**: Windows Graphics Capture (WGC) + Direct3D 11 + DXGI。
- **胶水层封装**: 通过 [`src/wgc_capture.cpp`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/src/wgc_capture.cpp) 和 [`src/capture.rs`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/src/capture.rs) 提供低延迟、无黑边的现代 Windows 窗口截屏能力。
- **自适应与容错**: 支持最小化窗口前置还原 (`--restore` / `--no-restore`)，以及帧获取超时控制 (`--timeout-ms`)、延时捕获 (`--delay-ms`)。

### 2. 窗口枚举与多维过滤
- **源码文件**: [`src/window_enum.rs`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/src/window_enum.rs)
- **过滤维度**:
  - 窗口标题模糊/精确匹配 (`--title`, `--exact`)
  - 窗口句柄 (`--hwnd`)
  - 进程 ID (`--pid`)
  - 进程名称 (`--process`)
  - 窗口类名 (`--class-name`)
- **严格排重**: 支持 `--require-unique`，当多窗口重名时安全报错，避免抓错目标。

### 3. Server / Client IPC 远程调用机制
- **源码文件**: [`src/server.rs`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/src/server.rs)、[`src/client.rs`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/src/client.rs)
- **通信方式**: Windows Named Pipe（默认管道名 `\\.\pipe\wgccli`）。
- **应用场景**: 专为 AI Agent（跨会话/隔离环境）或第三方自动化脚本设计。主用户会话运行服务端，客户端可通过发送 JSON 格式请求执行窗口枚举或截屏任务。

### 4. 诊断与格式化输出
- **自检工具**: 提供 `--doctor` 选项，快速检测本地 D3D11 设备及 WGC 运行时就绪状态 ([`src/d3d.rs`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/src/d3d.rs))。
- **输出格式**: 支持标准控制台输出及 `--json` 机器可读格式 ([`src/output.rs`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/src/output.rs))。

### 5. 体积与性能优化
- 移除了 chrono、anyhow 等重量级依赖，精简为极简依赖组合。
- [`Cargo.toml`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/Cargo.toml) 针对 Release 配置了高级优化：
  ```toml
  [profile.release]
  opt-level = "z"
  lto = true
  codegen-units = 1
  panic = "abort"
  strip = true
  ```

---

## 三、 代码仓库与提交记录

- **当前分支**: `rust-rewrite`
- **最近提交**:
  - `369c9d6` - `feat: v1.1.0 - server/client mode, window filtering, doctor`
  - `f820e3f` - `chore: bump version to 1.0.1`
  - `e9241a6` - `perf: remove chrono/anyhow deps, add release optimizations`
  - `cb9a476` - `feat: Rust rewrite of wgc-cli with C++ WGC glue`
- **未提交文件**:
  - [`release-notes.md`](file:///C:/Users/IceLolly/Music/Rust/wgc-cli-rust/release-notes.md) (待跟踪/提交)

---

## 四、 常用命令与使用示例

### 1. 窗口列举与筛选
```bash
# 列出当前所有顶层可见窗口
wgccli.exe --list

# 结构化 JSON 列出
wgccli.exe --list --json
```

### 2. 窗口截图
```bash
# 按标题捕获记事本并输出到目标目录
wgccli.exe --title "Notepad" --out C:\Temp

# 按精确标题捕获，延时 500ms
wgccli.exe --title "Visual Studio Code" --exact --delay-ms 500 --out C:\Temp

# 按 PID / HWND 捕获
wgccli.exe --pid 1234 --out C:\Temp
wgccli.exe --hwnd 0x00123456 --out C:\Temp\shot.png
```

### 3. Server / Client 模式（AI Agent / IPC）
```bash
# 启动服务端（在交互式桌面会话中常驻）
wgccli.exe --server

# 客户端发起 JSON 查询请求
wgccli.exe --client "{\"action\":\"list\"}"

# 客户端发起截图请求
wgccli.exe --client "{\"action\":\"capture\",\"title\":\"Notepad\",\"out\":\"C:\\\\Temp\"}"
```

### 4. 环境自检
```bash
wgccli.exe --doctor
```

---

## 五、 后续改进与建议

1. **版本跟踪**: 将 `release-notes.md` 纳入 Git 版本控制。
2. **测试强化**: 增加单元测试（如针对 `Cli` 解析边界条件、JSON IPC 协议序列化校验）。
3. **CI / CD 打包**: 可配置 GitHub Actions 实现跨平台/自动化 Release 构建，自动产出优化后的独立 exe。
