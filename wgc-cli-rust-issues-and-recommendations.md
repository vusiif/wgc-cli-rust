# wgc-cli-rust（Rust）源码问题清单与优化建议

> 仓库：[vusiif/wgc-cli-rust](https://github.com/vusiif/wgc-cli-rust)  
> 审查范围：`rust-rewrite` 分支公开源码；核查日期：2026-10-15  
> 当前 CLI 名称：`rgc-cli`。主要依赖 `xcap`、`image`、`clap`、`serde` 与 `arboard`，支持显示器截图、区域裁剪、PNG/JPEG/WebP、stdout、剪贴板及 JSON 输出。

## 0. 定位与原则

Rust 项目目前是**跨平台显示器截图 CLI**，不能把它当成 C++ `wgc-cli` 的完整等价实现：当前没有 C++ 版已有的窗口筛选、Named Pipe、MCP stdio 与 Windows 环境诊断等入口。

下面将“已确认的正确性问题”“工程/功能缺口”“性能优化假设”分开记录。优先级 **P0** 应先修，**P1** 为近期迭代，**P2** 为架构和性能优化。所有性能结论均需实测。

## 1. 总览

| 编号 | 优先级 | 问题或机会 | 分类 | 主要位置 |
|---|---|---|---|---|
| R-01 | P0 | `--json` 与 `-o -` 同时输出，污染图片二进制流 | 已确认的输出冲突 | `src/main.rs` |
| R-02 | P0 | 区域边界 `u32` 加法可能溢出 | 已确认的算术风险 | `src/capture.rs` |
| R-03 | P1 | 默认文件名只有秒精度，可能覆盖同秒截图 | 已确认的覆盖风险 | `src/output.rs` |
| R-04 | P1 | 不存在的目录无法可靠按目录语义处理 | 已确认的接口限制 | `src/output.rs` |
| R-05 | P1 | `--json` 模式的失败结果仍不是统一 JSON 对象 | 已确认的协议不一致 | `src/main.rs` |
| R-06 | P1 | 单元测试依赖真实显示器与剪贴板 | 已确认的 CI 障碍 | `src/capture.rs`、`src/output.rs` |
| R-07 | P1 | 尚无指定窗口截图及按标题/PID 选窗能力 | 功能缺口 | `src/capture.rs`、`src/cli.rs` |
| R-08 | P2 | 尚无与 C++ 版本对等的 MCP、Pipe/会话代理层 | 功能/架构缺口 | 当前 CLI 架构 |
| R-09 | P2 | 先整屏截图再 `.to_image()` 裁剪，非端到端零拷贝 | 已确认的复制路径 | `src/capture.rs` |
| R-10 | P2 | stdout 图片先完整编码到 `Vec` | 已确认的内存开销 | `src/output.rs` |
| R-11 | P2 | DPI、坐标系与多屏负坐标的契约需要明确 | 兼容性风险 | `src/capture.rs`、`src/cli.rs` |
| R-12 | P2 | 跨平台权限诊断、发布验证与性能基准不完善 | 工程增强建议 | `README.md`、工程配置 |

## 2. 逐项问题与可执行建议

### R-01｜P0｜禁止二进制图片与 JSON 混写 stdout

**证据：** `run()` 在 `is_stdout` 为真时调用 `write_to_stdout()`；之后如果 `args.json` 为真，又调用 `println!()` 输出 JSON。当前 CLI 定义没有看到阻止这两个选项同时出现的冲突声明。

**影响：** `rgc-cli -o - --json` 的标准输出先是 PNG/JPEG/WebP 字节，后面跟 JSON 文本，既不是完整有效的图片数据，也不是单一 JSON 文档。

**建议（推荐最小修复）：**
1. 在 clap 的参数约束或 `run()` 最前面禁止 `--json` 与 `--output -` 同时使用，并返回参数错误。
2. 明确规定：stdout 在 `-o -` 模式下**只包含二进制图像**；日志、进度、诊断走 stderr。
3. 若将来希望双通道元信息，可设计显式 `--metadata-stderr`，不要偷偷改变 `--json` 原语义。

**验收：**
- `rgc-cli -o - > image.png` 输出可被图片解码器正常识别。
- `rgc-cli -o - --json` 在截图前被拒绝，stdout 不产生图片或 JSON 混合内容。
- `rgc-cli -o shot.webp --json` 只输出一份合法 JSON。

**源码：** [main.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/main.rs)、[cli.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/cli.rs)

### R-02｜P0｜区域边界安全计算及缩放因子验证

**证据：** `logical_to_physical_region()` 用 `phys_x + phys_w > monitor_width`（Y 方向同理）判断边界，参与加法的数值均为 `u32`。逻辑坐标乘以 `f32 scale_factor` 后又强制转换为整数。

**影响：** 极端尺寸下，Debug 构建可能触发整数溢出 panic；Release 构建可能回绕并通过错误的边界判断。无效 DPI 比例、浮点精度或饱和转换也值得单独处理。

**建议：**
1. 检查 `scale_factor.is_finite() && scale_factor > 0.0`；对转换前范围进行检查。
2. 先验证起点在画面内，再使用 `phys_w <= monitor_width - phys_x`、`phys_h <= monitor_height - phys_y`，或 `checked_add`。
3. 确保宽高大于零；如需四舍五入，应以明确、一致的规则将逻辑矩形映射到物理矩形。
4. 将纯区域计算函数单测与真实截图集成测试分开。

**验收用例：** 正常矩形、贴边矩形、零宽高、负起点、超出画面、`u32::MAX`、`NaN`/无穷/非正比例系数。所有无效输入都应返回 `Err`，不得 panic 或回绕。

**源码：** [capture.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/capture.rs)

### R-03｜P1｜防止自动文件名碰撞与覆盖

**证据：** `generate_default_filename()` 使用 `%Y-%m-%d_%H-%M-%S`，精度只到秒，保存时直接 `save_with_format()`。

**影响：** 频繁截图或并发运行实例可能写到同一路径并覆盖此前文件。

**建议：**
1. 为自动生成的名称追加更高精度时间和短随机/唯一标识；不要只依赖毫秒避免碰撞。
2. 对自动生成的路径采取独占创建（如 `create_new(true)`）和碰撞重试；**明确用户指定的文件路径仍可按原有覆盖策略处理**。
3. 增加并发和同秒保存测试，不要求依赖真实桌面截图。

**验收：** 短时间内连续或并行生成的自动文件不会覆盖彼此；目录内生成文件数量与成功调用次数一致。

**源码：** [output.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/output.rs)

### R-04｜P1｜文件输出与目录输出的语义不明确

**证据：** `resolve_output_path()` 只在 `path.is_dir()` 为真时追加自动文件名；一个尚不存在的目录、某些带末尾分隔符的路径可能被当作文件名。

**建议：**
1. 优先引入明确选项：`--output-file` 和 `--output-dir`；保留 `--output` 兼容路径。
2. 对 `--output` 定义清楚的兼容规则：已存在目录→目录；已存在文件→文件；不存在路径→明确按扩展名判断，或提示用户指定类型。
3. 显式 `--format` 与文件扩展名不一致时给出警告或严格模式错误。
4. 在写入前检查目标类型、父目录权限以及路径错误，避免不必要的部分文件残留。

**验收：** 已存在目录、不存在目录、无扩展名文件名、以点结尾路径、权限不足、显式格式与扩展名冲突等场景都有明确的结果。

**源码：** [output.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/output.rs)

### R-05｜P1｜统一 JSON 错误响应及退出码

**证据：** 当前成功时序列化 `CaptureResultJson`，失败时经 `anyhow::Result` 返回；因此错误并未按同样的 JSON 结构输出。自动化调用方需要分别处理 stdout JSON 和 stderr 文本。

**建议：**
1. 抽象统一结果：`ok` / `errorCode` / `message` / `stage` / `suggestion` / `exitCode`；成功、失败都遵守公开 schema。
2. 区分 `BAD_ARGS`、`NO_MONITOR`、`PERMISSION_DENIED`、`CAPTURE_FAILED`、`ENCODE_FAILED`、`SAVE_FAILED`。
3. 定义 stderr 仅输出日志和人工诊断信息；JSON 模式的机器可读结果只占用 stdout。
4. 异常过程避免产生“看起来成功”但缺少实际图片的 JSON；提供版本化 schema。

**验收：** 无显示器、无权限、错误显示器索引、无法保存、编码失败时，JSON 均可正常解析且错误码稳定。

**源码：** [main.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/main.rs)、[capture.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/capture.rs)、[output.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/output.rs)

### R-06｜P1｜隔离硬件依赖测试，避免无桌面 CI 失败

**证据：** `capture.rs` 的 `#[test]` 直接调用 `Monitor::all()` 和 `capture_image()`；`output.rs` 的 `test_clipboard_copy()` 要求可用的系统剪贴板。它们不像区域解析/格式推断测试那样可以在普通无头环境运行。

**建议：**
1. 将纯函数与模拟后端的测试保留在默认 `cargo test` 集合。
2. 将桌面、剪贴板、Wayland/X11 授权相关测试改为显式启用的集成测试，例如标注 `#[ignore]` 或使用 feature gating。
3. 抽象 `CaptureBackend` / `ClipboardBackend` 等薄接口，利用可注入的假实现测试 CLI 错误处理。
4. CI 中按 Windows、macOS、Linux 做编译、fmt、clippy 与纯单测；真实截图测试在各自有图形会话的 Runner 运行。

**验收：** 无 DISPLAY、无交互桌面的 CI 仍能通过默认测试；图形测试有独立的执行命令与失败报告。

**源码：** [capture.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/capture.rs)、[output.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/output.rs)

### R-07｜P1｜实现窗口枚举与指定窗口截图

**证据：** 当前 CLI 只调用 `xcap::Monitor`，尚未暴露窗口枚举及截图入口。`xcap` 的 `Window::all()`、`Window::title()`、`Window::pid()`、`Window::capture_image()` 已可作为第一阶段基础。

**建议：**
1. 加入 `--list-windows [--json]`，返回窗口 ID、PID、标题、应用名、尺寸与最小化状态（可用字段依平台能力而定）。
2. 加入 `--window-id`、`--title`、`--pid`（互斥或组合规则写清）；默认要求唯一匹配，避免无声选择错误窗口。
3. 引入后端能力查询：不支持窗口截图的平台返回确定错误，不把 Linux/Wayland 标成无条件可用。
4. Windows 专有的 HWND、最小化恢复和跨会话服务，可在后续 Windows 后端进一步实现，不要求 xcap 一次解决。

**验收：** 在支持的平台上能按窗口 ID、标题及 PID 稳定截图；歧义会报错；受限环境提供可操作的诊断。

**源码：** [capture.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/capture.rs)、[cli.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/cli.rs)  
**上游接口：** [xcap `Window`](https://docs.rs/crate/xcap/latest/source/src/window.rs)

### R-08｜P2｜为 Agent 工作流补足 MCP 与 Windows IPC

**现状：** Rust CLI 有 JSON 输出，但当前没有像 C++ 工程那样的 MCP stdio Server 或 Windows Named Pipe server/client。

**建议：**
1. 复用单一核心截图服务接口（参数、结果、错误），将 CLI、MCP、IPC 作为外层适配器。
2. MCP 优先提供 `list_monitors`、`list_windows`、`capture_window`；支持可选图片内容返回和尺寸上限。
3. Windows 会话隔离场景再加 Named Pipe 服务，并继承 C++ 项目的最小权限 DACL、拒绝远程访问、超时和请求限制。
4. 非 Windows 平台不宜强行模拟相同权限模型；应按 OS 原生授权机制分别实现。

**验收：** 在 Windows AI Agent 场景能够以 MCP 直接取得目标窗口图片；CLI 与 MCP 的匹配、错误和输出尺寸语义一致。

**参考：** [C++ 功能说明](https://github.com/vusiif/wgc-cli)、[MCP Tools 规范](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)

### R-09｜P2｜裁剪并非端到端零拷贝

**证据：** `capture_screen()` 先调用 `Monitor::capture_image()` 取得整屏 RGBA，再经 `crop_imm(...).to_image()` 创建区域副本。`crop_imm` 可产生视图，但 `to_image()` 明确生成新的拥有型缓冲区。

**建议：**
1. README 避免笼统使用“Zero-copy”描述整个管线。
2. 首先评估 xcap/各平台后端是否能原生捕获 ROI；可以做到时尽量省去整屏取图。
3. 若依旧需要整屏，尽可能直接从裁剪视图向编码器输出，减少中间完整副本（需确认具体编码 API 能否接受行跨度）。
4. 性能优化前先建立“整屏、1/4 区域、小区域”的可重复基准。

**验收：** 内存分配数量、峰值内存与时延有客观下降，裁剪结果的像素、坐标、边界与原实现一致。

**源码：** [capture.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/capture.rs)

### R-10｜P2｜stdout 的编码缓冲与峰值内存

**证据：** `write_to_stdout()` 把完整图片编码到 `Cursor<Vec<u8>>`，随后 `write_all()` 到 stdout。因而“stdout streaming”目前指输出到管道，而非整个编码过程流式、恒定内存。

**建议：**
1. 评估 PNG/JPEG/WebP 各编码器是否允许对只支持 `Write` 的 stdout 直接编码；接口需要 `Seek` 时保留必要缓冲。
2. 对可流式输出的格式减少整图压缩结果的驻留副本，同时保持二进制 stdout 纯净。
3. 对 broken pipe 与消费者提前退出提供正确错误处理。

**验收：** 对相同图像和格式，stdout 与落盘文件可成功解码；峰值内存不升高，broken pipe 退出可预期。

**源码：** [output.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/output.rs)

### R-11｜P2｜明确坐标系与 DPI 语义

**证据：** CLI 将 `--region` 描述为“logical coordinates”，实现禁止负 `x/y`，随后乘显示器 `scale_factor` 得到像素坐标。这表示当前契约更接近“所选显示器本地逻辑坐标”，**并不支持任意全局虚拟桌面负坐标**。

**建议：**
1. README 明确 `--region` 的原点、是否相对所选显示器、逻辑/物理像素含义。
2. 提供 `--coords logical|physical` 或至少稳定一个无歧义的默认坐标模式。
3. 增加比例为 1.0/1.25/1.5/2.0、异构 DPI、多显示器左侧负全局坐标、贴边裁剪测试。
4. 检查 xcap 后端实际返回的像素宽高与监视器报告值是否一致，避免后端差异导致裁剪错位。

**验收：** 同一区域在不同 DPI 下坐标契约清晰，预期区域不会偏移或越界。

**源码：** [capture.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/capture.rs)、[cli.rs](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/src/cli.rs)

### R-12｜P2｜跨平台可用性诊断、打包与发布

**建议：**
1. 增加 `--doctor --json`：显示 OS、会话/显示服务器、截图授权、主显示器与后端可用能力。
2. 区分 Linux X11、Wayland 及其权限机制；不要保证所有 Wayland 环境都能以同样方式完成窗口截图。
3. 增加 `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`、跨平台构建矩阵、锁定依赖审查。
4. 将 README 中诸如“编码结果比 PNG 小约 30%”“可执行文件约 2.2 MB”的描述改成**带测试条件的示例数据**，而非泛化承诺。
5. 发布资产提供版本、平台架构、SHA-256 和简单的安装示例。

**验收：** 不支持的图形环境返回准确说明；每个平台的发布包经过可重现的构建和基本自检。

**参考：** [README](https://github.com/vusiif/wgc-cli-rust)、[Cargo.toml](https://github.com/vusiif/wgc-cli-rust/blob/rust-rewrite/Cargo.toml)、[xcap 说明](https://docs.rs/crate/xcap/latest)

## 3. 推荐 PR 拆分

- [ ] **PR-1：正确性热修复**（R-01、R-02）— 修复 stdout 冲突、算术边界及纯单元测试。
- [ ] **PR-2：文件与错误契约**（R-03、R-04、R-05）— 唯一文件名、路径语义、JSON 错误。
- [ ] **PR-3：测试可移植性**（R-06、R-11）— 纯单测/图形集成测试分离、DPI 矩阵。
- [ ] **PR-4：窗口能力**（R-07）— 枚举窗口、筛选、截图及平台能力检测。
- [ ] **PR-5：Agent 接口**（R-08）— MCP、图片返回，随后评估 Windows Pipe。
- [ ] **PR-6：性能优化**（R-09、R-10、R-12）— 先 benchmark 再改造复制/编码路径和发布流程。

## 4. 最小验收测试清单

| 场景 | 预期 |
|---|---|
| `rgc-cli -o - --json` | 在产生二进制流之前拒绝不兼容参数 |
| `rgc-cli -o - > shot.png` | 文件可作为独立 PNG 解码 |
| 区域终点正好贴右/下边界 | 成功，且尺寸准确 |
| 区域尺寸使用极大无符号整数 | 返回范围错误，不 panic |
| 未检测到显示器，且要求 `--json` | 返回合法 JSON 错误对象 |
| 同一秒内密集自动保存 | 不覆盖已有自动生成文件 |
| 非交互式 CI 执行默认测试 | 不需要桌面或剪贴板 |
| 不支持窗口截图的后端 | 明确报错，而非静默退回整屏 |

## 5. 性能基准建议

请分别统计**枚举显示器、捕获整屏、区域转换、裁剪复制、编码、文件写入、stdout 输出、剪贴板写入、端到端耗时和峰值 RSS**。按操作系统/显示服务器、DPI、图像格式和裁剪区域大小进行分组。对同一图片数据进行编码对比，避免截图内容变化影响 PNG/WebP 大小和耗时。

**注意：** 本报告来自在线公开源码静态审查；没有在三种操作系统上实际编译、截图或运行性能基准。因此性能提升建议是实验方案，不是已测得的收益。
