# AGENTS.md

本文件为在当前仓库中工作的 AI coding agents 提供项目级指引。

## 项目概览

- 项目名：`port-kill`
- 类型：桌面应用
- 技术栈：`Tauri 2 + Vue 3 + TypeScript + Vite + Rust`
- 目标用途：扫描本机 TCP 监听端口，展示端口对应进程，并支持结束或强制结束进程
- 当前定位：开发场景下的端口占用排查工具

## 平台边界

当前后端实现只支持 `macOS`。

原因是 Rust 后端直接依赖以下系统命令与路径：

- `/usr/sbin/lsof`
- `/bin/ps`
- `/bin/kill`

因此：

- 可以继续优化 macOS 体验和稳定性
- 不要默认把后端逻辑当作跨平台实现
- 如果要支持 Linux 或 Windows，需要显式设计平台分支，不要在现有逻辑上做隐式兼容假设

## 常用开发命令

安装依赖：

```bash
pnpm install
```

前端开发：

```bash
pnpm dev
```

桌面应用开发：

```bash
pnpm tauri dev
```

前端构建与类型检查：

```bash
pnpm build
```

桌面应用构建：

```bash
pnpm tauri build
```

版本号同步脚本：

```bash
pnpm version:set
```

## 代码结构

前端主要目录：

- `src/App.vue`：主题提供与全局 UI 外壳
- `src/components/PortKillWorkbench.vue`：主工作区，负责加载、刷新、结束进程、回弹检测
- `src/components/PortToolbar.vue`：搜索、排序、刷新、主题切换
- `src/components/PortTable.vue`：端口表格
- `src/components/PortDetailPanel.vue`：进程详情面板
- `src/types.ts`：前后端共享的数据结构定义
- `src/portHints.ts` / `src/portMeta.ts`：常见开发端口提示信息

后端主要目录：

- `src-tauri/src/lib.rs`：Tauri commands、端口扫描、进程信息解析、结束进程逻辑
- `src-tauri/src/main.rs`：Tauri 启动入口
- `src-tauri/tauri.conf.json`：桌面窗口与打包配置

## 关键后端行为

当前暴露的核心 Tauri commands 包括：

- `list_ports`：扫描 `TCP LISTEN` 端口，聚合进程信息并返回给前端
- `kill_process`：发送 `TERM` 或 `KILL` 信号，并轮询确认退出
- `check_port_rebound`：检查端口是否被新的进程重新占用

修改后端时请注意：

- 当前只处理 `TCP LISTEN`，不要误改成包含 UDP 或已建立连接，除非这是明确需求
- 进程展示是按 `pid + protocol + port` 聚合的，不要轻易破坏这个聚合键
- 命令执行带超时控制，新增系统命令时应保持类似的超时与错误处理模式
- 返回给前端的错误信息当前以中文为主，新逻辑应保持一致风格

## 关键前端行为

- 默认自动刷新间隔当前为 `8s`，定义在 `src/components/PortKillWorkbench.vue`
- 首次加载、手动刷新、静默刷新、kill 后刷新、端口回弹检查都由 `PortKillWorkbench.vue` 协调
- 排序支持最近启动时间、端口、进程名
- 搜索会匹配端口、PID、进程名、命令、工作目录、监听地址摘要

修改前端时请注意：

- `src/types.ts` 必须与 Rust 返回结构保持一致
- 如果改动 Tauri command 返回字段，必须同步更新前端类型与消费逻辑
- 不要把加载态、错误态、kill 中状态拆散到多个组件，当前状态管理集中在主工作区组件，保持这个结构更容易维护

## 变更约束

- 优先做小而确定的改动，不要无关重构
- 不要引入仅为“代码更漂亮”但没有实际收益的抽象
- 涉及 Rust 后端命令时，优先保证稳定性、超时控制、错误信息可读性
- 涉及进程结束逻辑时，默认把“误杀风险”和“权限不足”当作一等问题处理
- 若新增依赖，确认它对 Tauri 桌面打包与 macOS 运行没有额外副作用

## 测试与验证建议

完成改动后，优先按改动范围执行对应验证：

- 前端类型与构建验证：`pnpm build`
- 桌面联调验证：`pnpm tauri dev`

如果改动影响以下场景，至少手动验证一次：

- 端口列表能正常加载
- 排序和搜索仍然工作
- 普通结束与强制结束流程正常
- 端口被自动拉起时，回弹提示仍然出现
- 深色 / 浅色主题显示正常

## 文档与协作约定

- 面向这个仓库写说明时，优先使用中文
- 讨论库、框架、SDK、CLI、云服务或 API 用法时，先用 Context7 获取当前文档，再给结论
- 若改动会影响平台边界、命令行为或数据结构，记得同步更新 `README.md` 或相关说明文件
