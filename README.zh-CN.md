# Port Kill

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Release](https://img.shields.io/github/v/release/wungjyan/port-kill)](https://github.com/wungjyan/port-kill/releases)
[![License](https://img.shields.io/github/license/wungjyan/port-kill)](./LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey)](#平台支持)

Port Kill 是一款原生 macOS 工具，用于查找占用 TCP 监听端口的进程，并在执行结束操作前复核进程身份。

它适用于本地开发中的端口冲突场景：服务器、watcher 或后台服务持续占用端口，而直接执行 `kill` 又可能误操作已经过期或无关的 PID。

项目基于 Tauri 2、Vue 3、TypeScript、Vite 和 Rust 构建。

## 为什么使用 Port Kill？

`lsof`、`ps` 和 `kill` 等命令行工具功能强大，但排查一次端口冲突通常需要手动组合多条命令。Port Kill 将相关进程信息集中展示，并在发送信号前重新验证目标。

## 主要功能

- 展示 TCP 监听端口、进程名、PID、命令、工作目录、监听地址与启动时间。
- 按端口、PID、进程名、命令、工作目录或监听地址搜索。
- 按最近启动时间、端口或进程名排序。
- 区分仅本机、指定地址和所有网卡三类监听范围。
- 识别 Vite、Next.js、PostgreSQL、Redis 和 MongoDB 等常见开发端口。
- 普通结束发送 `TERM`，明确选择强制结束时发送 `KILL`。
- 操作前展示同一进程关联的其他监听端口。
- 检测 watcher 或服务管理器在原进程退出后重新占用端口的情况。
- 支持可配置的自动刷新；扫描失败时保留上次成功结果。
- 跟随系统外观，也可持久化浅色或深色主题偏好。

## 安全设计

结束进程属于破坏性操作。Rust 后端会在发送信号前重新扫描选中的端口，并将 PID、所属用户、进程名和启动时间与界面中展示的数据进行比对，以降低列表过期或 PID 被复用时的误操作风险。

其他安全边界：

- 只允许结束当前用户所属的进程。
- 普通结束使用 `TERM`；强制结束使用不可拦截的 `KILL`，需要用户明确选择。
- 系统命令均带超时控制，超时后会终止并回收子进程。
- 端口与进程信息仅在本机处理。Port Kill 不包含遥测，也不会把扫描结果发送到远程服务。

这些检查可以降低风险，但无法消除所有竞态条件。结束进程前请核对进程详情，使用强制结束时尤其需要谨慎。

## 下载

请从 [GitHub Releases](https://github.com/wungjyan/port-kill/releases) 下载最新版本：

- Apple Silicon Mac：`Port.Kill_<version>_aarch64.dmg`
- Intel Mac：`Port.Kill_<version>_x64.dmg`

打开 DMG 后，将 **Port Kill.app** 拖入“应用程序”目录。

### macOS 提示应用“已损坏”

当前发行版尚未完成 Apple 签名与公证。从浏览器下载后，macOS 可能为应用添加隔离属性，而 Gatekeeper 无法验证其来源。

确认应用来自本仓库的 GitHub Releases 页面后，将其移动到“应用程序”目录并执行：

```bash
xattr -dr com.apple.quarantine "/Applications/Port Kill.app"
open "/Applications/Port Kill.app"
```

该命令只会移除此应用的隔离属性。请勿对来源不可信的副本执行。

## 平台支持

Port Kill 当前仅支持 **macOS**。后端依赖以下系统命令与路径：

- `/usr/sbin/lsof`
- `/bin/ps`
- `/bin/kill`

项目只扫描 `TCP LISTEN` 套接字，暂不包含 UDP 套接字或已建立的 TCP 连接。

## 本地开发

### 环境要求

- macOS
- Node.js 20 或更高版本
- pnpm 9 或更高版本
- Rust stable
- Xcode Command Line Tools

### 本地运行

```bash
pnpm install
pnpm tauri dev
```

只开发界面时可以使用 `pnpm dev`。浏览器模式无法调用 Tauri 后端命令。

### 验证改动

```bash
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

### 构建桌面应用

```bash
pnpm tauri build
```

## 项目结构

```text
src/                          Vue 前端
src/components/               端口列表、工具栏与进程详情
src-tauri/src/lib.rs          端口扫描、进程身份复核与结束逻辑
src-tauri/tauri.conf.json     桌面窗口、安全策略与打包配置
docs/frontend-api.md          前后端数据接口说明
docs/releasing.md             维护者发布清单
.github/workflows/release.yml 双架构 macOS 发布工作流
```

## 发布

推送 `v*` 标签后，GitHub Actions 会检查标签是否与 JavaScript、Tauri 和 Rust 配置中的版本一致，然后分别构建 Apple Silicon 与 Intel 发行产物。

详细流程见[发布清单](./docs/releasing.md)，版本变化见[更新日志](./CHANGELOG.md)。

## 参与贡献

欢迎提交缺陷报告和范围明确的 Pull Request。如果计划调整核心行为或增加平台支持，请先创建 Issue，以便讨论改动范围与安全影响。

涉及进程结束流程的改动应保留目标身份复核、当前用户限制、命令超时和清晰的错误提示。

## 许可证

Port Kill 基于 [MIT License](./LICENSE) 开源。
