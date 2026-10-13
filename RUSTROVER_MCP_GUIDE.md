# RustRover MCP Server 安装、配置与调用完整指南

本指南详细介绍如何在 **JetBrains RustRover** 中安装并启用官方 **MCP (Model Context Protocol) Server**，以及如何将它接入各种 AI 编程助手（如 Antigravity、Claude Desktop、Cursor 等）实现由 AI 驱动的 IDE 自动化操作。

---

## 一、 什么是 RustRover MCP Server？

[Model Context Protocol (MCP)](https://modelcontextprotocol.io/) 是一套开放协议。通过在 RustRover 中运行 MCP Server，外部的 AI Agent / 大模型可以直接操作您的 IDE，例如：
- 获取当前编辑器打开的文件与光标位置
- 触发 IDE 项目构建并实时接收编译与代码检查问题 (`build_project`, `get_file_problems`)
- 在 IDE 标签页中直接打开指定文件或跳转到指定行 (`open_file_in_editor`)
- 获取项目结构、模块依赖、Git 状态与终端执行

---

## 二、 第一步：在 RustRover 中安装与启用插件

### 1. 检查或安装插件
> **提示**：自 2025.2 版本起，JetBrains IDE 已原生预装 **MCP Server** 插件。

1. 打开设置：快捷键 `Ctrl + Alt + S`（macOS 为 `Cmd + ,`）。
2. 在左侧导航栏中点击 **Plugins**（插件）。
3. 检查 **Installed**（已安装）标签页：
   - 搜索 `MCP Server`。若已存在，确保其状态为 **已勾选启用**。
4. 如果未预装：
   - 切换到 **Marketplace** 标签页；
   - 搜索 `MCP Server`（供应商为 **JetBrains s.r.o.**）；
   - 点击 **Install** 安装，并按提示重启 IDE。

---

### 2. 在 IDE 设置中开启服务
1. 打开设置（`Ctrl + Alt + S`）。
2. 进入 **Tools**（工具） > **MCP Server**。
3. 勾选 **Enable MCP Server**（启用 MCP 服务）。
4. 此时会弹出安全权限提示对话框（授予外部 AI 工具读取工程、运行终端、构建等权限），点击 **Enable / 允许**。
5. 点击底部 **Apply** 和 **OK** 保存。

---

## 三、 第二步：将 RustRover 接入 AI 客户端

根据所使用的 AI 工具类型，接入方式分为以下三种：

### 方式 A：IDE 内一键自动配置（Auto-Configuration，最简便）
1. 打开 **Settings** > **Tools** > **MCP Server**。
2. 在页面下方的 **Clients Auto-Configuration**（客户端自动配置）区域中；
3. IDE 会自动检测本机已安装的 AI 客户端（如 Claude Code、OpenCode、VS Code 等）；
4. 找到对应客户端，点击 **Auto-Configure**（自动配置），IDE 会直接将连接配置写入对应软件的配置文件中。

---

### 方式 B：SSE 直连方式（适用于支持 HTTP/SSE 的客户端）
JetBrains MCP 默认在本地启动 HTTP/SSE 监听（常见默认端口如 `64342` 或 `63342`）。

1. 在 **Settings** > **Tools** > **MCP Server** 面板中，点击 **Copy SSE Config** 按钮；
2. 此时剪贴板将获取到类似如下的配置：
   ```json
   {
     "type": "sse",
     "url": "http://127.0.0.1:64342/sse"
   }
   ```
3. 将该 URL 配置填入对应的 AI 客户端。例如在 **Antigravity / Gemini CLI** 的 `~/.gemini/config/mcp_config.json` 中配置：
   ```json
   {
     "mcpServers": {
       "rustrover": {
         "url": "http://127.0.0.1:64342/sse"
       }
     }
   }
   ```

---

### 方式 C：通过 `@jetbrains/mcp-proxy` 桥接（适用于仅支持 stdio 的客户端）
部分桌面端工具（如 **Claude Desktop**）仅支持基于命令行标准输入输出（`stdio`）的通信协议。JetBrains 官方提供了 npm 代理包自动转发到当前打开的 IDE 实例。

在客户端配置文件（如 `claude_desktop_config.json`）中添加：

```json
{
  "mcpServers": {
    "rustrover": {
      "command": "npx",
      "args": ["-y", "@jetbrains/mcp-proxy"]
    }
  }
}
```

> **注意**：使用此方式需要本机安装有 Node.js 和 `npx` 环境。

---

## 四、 常见客户端配置文件路径

| 客户端名称 | 操作系统 | 配置文件路径 |
| :--- | :--- | :--- |
| **Antigravity / Gemini CLI** | Windows | `C:\Users\<用户名>\.gemini\config\mcp_config.json` |
| **Claude Desktop** | Windows | `%APPDATA%\Claude\claude_desktop_config.json` |
| **Claude Desktop** | macOS | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| **OpenCode** | 通用 | `~/.config/opencode/opencode.json` |

---

## 五、 RustRover MCP 常用工具能力清单

连接建立后，AI 助手将获得对 RustRover 的深度调用权限，常用工具包含：

| 工具名称 | 功能描述 |
| :--- | :--- |
| `build_project` | 触发 IDE 构建当前项目或指定文件，返回构建报错与警告 |
| `get_all_open_file_paths` | 查询 IDE 中当前所有已打开的编辑器文件标签页 |
| `open_file_in_editor` | 驱动 IDE 在前台打开指定代码文件 |
| `get_file_problems` | 运行 IDE 代码检查分析，获取语法错误、Lint 告警 |
| `get_run_configurations` | 获取项目当前配置好的 Cargo / 调试运行配置 |
| `execute_terminal_command` | 在 IDE 内部终端窗口中运行命令 |
| `git_status` | 读取当前项目在 IDE 中的 Git 状态与改动文件列表 |

---

## 六、 常见问题与排查 (Troubleshooting)

1. **连接报 403 / 拒绝连接**:
   - 确认在 RustRover 的 `Tools` > `MCP Server` 中已勾选 **Enable MCP Server**；
   - 检查是否有弹窗提示安全授权未点击允许。
2. **多 IDE 实例共存问题**:
   - 如果同时运行了 IntelliJ IDEA、PyCharm 和 RustRover，`@jetbrains/mcp-proxy` 可能会连接到最先启动的 IDE；
   - 建议通过复制 RustRover 专属的 **SSE URL** 精确绑定到 RustRover 实例。
3. **IDE 重启后端口变化**:
   - 若端口动态变化，可在设置界面固定端口或使用官方 `@jetbrains/mcp-proxy` 自动发现。
