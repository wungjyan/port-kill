# Port Kill

Port Kill 是一款面向开发场景的 macOS 桌面工具，用于扫描本机 TCP 监听端口、定位占用进程，并安全地发送结束或强制结束信号。

项目基于 `Tauri 2 + Vue 3 + TypeScript + Vite + Rust` 构建。

## 主要功能

- 展示端口、进程名、PID、命令、工作目录、监听地址与启动时间
- 搜索端口、PID、进程名、命令、工作目录和监听地址
- 按最近启动时间、端口或进程名排序
- 标识仅本机、指定地址和所有网卡三类监听范围
- 提供常见开发端口提示，例如 Vite、Next.js、PostgreSQL、Redis 和 MongoDB
- 支持普通结束与强制结束，并提示同一进程关联的其他监听端口
- 操作前复核 PID、端口、用户、进程名和启动时间，降低误杀风险
- 结束后分阶段检查端口是否被 watcher 或后台服务重新拉起
- 自动刷新失败时保留上次成功数据；刷新间隔可在设置中调整
- 跟随系统主题，也可手动切换并持久化偏好

## 下载与平台支持

发行版可从 [GitHub Releases](https://github.com/wungjyan/port-kill/releases) 下载，提供 Apple Silicon 与 Intel 两种 macOS 架构产物。

当前后端固定调用 macOS 的 `/usr/sbin/lsof`、`/bin/ps` 和 `/bin/kill`，因此不支持 Linux 或 Windows。应用只扫描 `TCP LISTEN`，不包含 UDP 或已建立连接。

## 安全边界

- 只允许结束当前用户所属的进程，系统进程和其他用户进程不可操作。
- 进程列表可能在刷新间隔内发生变化；后端会在发送信号前重新验证目标身份。
- 普通结束发送 `TERM`，允许进程清理资源；强制结束发送不可拦截的 `KILL`，应谨慎使用。
- 系统命令均带超时控制，超时后会主动终止并回收子进程。

## 本地开发

环境要求：macOS、Node.js 20、pnpm 9、Rust stable，以及 Xcode Command Line Tools。

```bash
# 安装依赖
pnpm install

# 启动桌面开发模式
pnpm tauri dev

# 前端类型检查与生产构建
pnpm build

# 构建当前 Mac 架构的安装包
pnpm tauri build
```

仅调试前端界面时，可运行 `pnpm dev`；浏览器环境无法调用 Tauri 后端命令。

## 项目结构

```text
src/                         Vue 前端
src/components/              端口列表、工具栏与详情组件
src-tauri/src/lib.rs         端口扫描、进程校验与结束逻辑
src-tauri/tauri.conf.json    窗口、安全策略与打包配置
docs/frontend-api.md         前后端数据接口说明
.github/workflows/release.yml 双架构 macOS 发布工作流
```

## 发布

推送 `v*` 标签后，GitHub Actions 会校验标签与项目版本是否一致，并分别构建 Apple Silicon 和 Intel 发行产物。完整步骤见 [发布清单](./docs/releasing.md)，版本变化见 [CHANGELOG](./CHANGELOG.md)。
