# 前端数据接口说明

本文档整理 Rust 后端通过 Tauri 暴露给前端的全部数据接口，方便后续独立开发或重构 UI。

当前接口定义来源：

- 后端注册位置：`src-tauri/src/lib.rs`
- 前端类型定义：`src/types.ts`
- 调用方式：`@tauri-apps/api/core` 的 `invoke`

## 总览

当前 Rust 后端注册了 3 个 Tauri command：

| Command | 用途 | 主要 UI 场景 |
| --- | --- | --- |
| `list_ports` | 获取当前 TCP LISTEN 端口列表 | 首次加载、手动刷新、自动刷新、kill 后刷新 |
| `kill_process` | 结束或强制结束指定 PID | 点击“结束”“强制”操作 |
| `check_port_rebound` | 检查端口是否被新进程重新占用 | kill 成功后的回弹提示 |

所有返回字段都会经过 `serde(rename_all = "camelCase")` 转成前端常用的 camelCase。

## 1. `list_ports`

扫描当前 macOS 上所有 `TCP LISTEN` 端口，聚合端口、进程、监听地址、命令行、工作目录、启动时间等信息。

### 调用

```ts
import { invoke } from "@tauri-apps/api/core";
import type { PortListResponse } from "../types";

const response = await invoke<PortListResponse>("list_ports");
```

### 参数

无。

### 返回类型

```ts
export type PortListResponse = {
  currentUser: string;
  items: PortProcess[];
};
```

### 字段说明

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `currentUser` | `string` | 当前登录用户，来自环境变量 `USER`，失败时为 `"unknown"` |
| `items` | `PortProcess[]` | 端口进程列表 |

### `PortProcess`

```ts
export type PortProcess = {
  pid: number;
  processName: string;
  user: string;
  protocol: string;
  port: number;
  state: string;
  hosts: string[];
  hostSummary: string;
  ipVersions: string[];
  command: string;
  cwd: string | null;
  startedAt: string | null;
  startedAtTs: number | null;
};
```

| 字段 | 类型 | 说明 | UI 建议 |
| --- | --- | --- | --- |
| `pid` | `number` | 监听端口的进程 PID | 可作为操作目标、行辅助信息 |
| `processName` | `string` | 后端从完整命令中推导出的进程名 | 表格主标题，例如 `node`、`vite`、`ControlCenter` |
| `user` | `string` | 进程所属用户 | 和 `currentUser` 不同时可显示权限/用户提示 |
| `protocol` | `string` | 协议，目前主要为 `TCP` | 可隐藏，或作为高级信息展示 |
| `port` | `number` | 监听端口号 | UI 核心字段，建议高优先级展示 |
| `state` | `string` | TCP 状态，当前扫描目标通常是 `LISTEN` | 可隐藏，或在详情中展示 |
| `hosts` | `string[]` | 原始监听地址列表，例如 `["127.0.0.1", "*"]` | 详情面板展示完整监听地址 |
| `hostSummary` | `string` | 后端生成的地址摘要，例如 `IPv4 · 127.0.0.1` | 表格地址列展示 |
| `ipVersions` | `string[]` | IP 版本列表，例如 `["IPv4"]` 或 `["IPv4", "IPv6"]` | 可做小标签或次级文本 |
| `command` | `string` | 完整命令行 | 区分多个同名进程的关键字段，建议 tooltip 或详情展示 |
| `cwd` | `string \| null` | 进程工作目录；权限不足或读取失败时为 `null` | 区分多个开发项目的关键字段，可显示路径尾部 |
| `startedAt` | `string \| null` | 进程启动时间，来自 `ps lstart`，例如 `Thu May 21 14:10:00 2026` | 可格式化后展示 |
| `startedAtTs` | `number \| null` | 后端生成的可排序时间数值，格式类似 `YYYYMMDDHHMMSS` | UI 排序推荐使用 |

### 后端排序规则

后端默认返回列表已排序：

1. `startedAtTs` 越新越靠前
2. `port` 越小越靠前
3. `processName` 字典序
4. `pid` 越小越靠前

前端可以继续二次排序，但首次展示可以直接使用返回顺序。

### 聚合规则

后端按以下键聚合：

```text
pid + protocol + port
```

同一个进程在同一端口上监听多个地址时，不会返回多行，而是合并到同一个 `PortProcess` 的 `hosts` 和 `ipVersions`。

### 失败情况

`invoke("list_ports")` 失败时会抛出 `string` 错误，常见情况：

| 错误特征 | 可能原因 | UI 建议 |
| --- | --- | --- |
| 包含 `timeout` | `lsof` 扫描超时 | 提示“端口扫描超时，请重试” |
| 包含 `lsof` | 无法执行或权限异常 | 提示“无法执行端口扫描命令，请检查系统权限” |
| 其他字符串 | 系统命令错误 | 展示原始错误或通用失败提示 |

## 2. `kill_process`

向指定 PID 发送结束信号，并轮询确认进程是否退出。

### 调用

```ts
import { invoke } from "@tauri-apps/api/core";
import type { KillResult } from "../types";

const result = await invoke<KillResult>("kill_process", {
  pid: item.pid,
  force: false,
  expectedPort: item.port,
  expectedStartedAtTs: item.startedAtTs,
  expectedProcessName: item.processName,
  expectedUser: item.user,
});
```

### 参数

```ts
type KillProcessArgs = {
  pid: number;
  force?: boolean;
  expectedPort: number;
  expectedStartedAtTs: number | null;
  expectedProcessName: string;
  expectedUser: string;
};
```

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `pid` | `number` | 是 | 要结束的进程 PID，必须大于 0 |
| `force` | `boolean` | 否 | `false` 或省略时发送 `TERM`；`true` 时发送 `KILL` |
| `expectedPort` | `number` | 是 | 列表中该进程监听的端口，用于操作前复核 |
| `expectedStartedAtTs` | `number \| null` | 是 | 列表中的启动时间标识，用于识别 PID 复用 |
| `expectedProcessName` | `string` | 是 | 列表中的进程名，用于身份复核 |
| `expectedUser` | `string` | 是 | 列表中的进程所属用户；当前只允许操作当前用户进程 |

### 返回类型

```ts
export type KillResult = {
  pid: number;
  signal: string;
  success: boolean;
  message: string;
};
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `pid` | `number` | 被操作的 PID |
| `signal` | `string` | 实际使用的信号，普通结束为 `TERM`，强制结束为 `KILL` |
| `success` | `boolean` | 是否成功确认进程退出；成功返回时通常为 `true` |
| `message` | `string` | 后端消息，成功时当前为 `已结束` |

### 行为说明

后端会重新扫描目标端口，复核 PID、进程名、所属用户和启动时间，再用 `kill -0 <pid>` 检查进程是否存在，然后执行：

| `force` | 系统命令 | 含义 |
| --- | --- | --- |
| `false` / 省略 | `/bin/kill -TERM <pid>` | 普通结束，允许进程自行清理 |
| `true` | `/bin/kill -KILL <pid>` | 强制结束，进程无法拦截 |

发送信号成功后，后端会轮询确认进程是否真的退出。轮询最多 10 次，间隔从 50ms 逐步增加到 200ms。

### 失败情况

`invoke("kill_process")` 失败时会抛出 `string` 错误，常见情况：

| 错误文本 | 触发条件 | UI 建议 |
| --- | --- | --- |
| `无效的 PID` | `pid <= 0` | 不应出现，可视为开发错误 |
| `进程 {pid} 不存在` | kill 前进程已不存在 | 提示进程已退出，并刷新列表 |
| `进程身份已变化...` | 列表数据过期、目标端口或 PID 已变化 | 阻止操作并提示刷新列表 |
| `没有权限结束进程 {pid}...` | 系统返回 `Operation not permitted` | 提示权限不足 |
| `进程 {pid} 已退出` | 系统返回 `No such process` | 提示已退出，并刷新列表 |
| `进程 {pid} 未响应结束信号，请尝试强制结束` | `TERM` 后轮询仍存在 | 提示可尝试强制结束 |
| `进程 {pid} 在强制结束后仍未退出` | `KILL` 后轮询仍存在 | 提示强制结束失败 |

### UI 建议

- 普通结束和强制结束应有不同视觉层级。
- 强制结束建议二次确认。
- 操作期间建议根据 `pid` 做行级 loading，避免重复点击。
- 成功后建议立即静默刷新端口列表。
- 成功后可以延迟调用 `check_port_rebound` 检查端口是否被重新监听。

## 3. `check_port_rebound`

检查某个端口在 kill 后是否被新的进程重新占用，用于识别 watcher、后台服务、进程管理器等自动拉起场景。

### 调用

```ts
import { invoke } from "@tauri-apps/api/core";
import type { ReboundCheckResult } from "../types";

const result = await invoke<ReboundCheckResult>("check_port_rebound", {
  port: item.port,
  previousPid: item.pid,
  previousProcessName: item.processName,
});
```

### 参数

```ts
type CheckPortReboundArgs = {
  port: number;
  previousPid: number;
  previousProcessName: string;
};
```

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `port` | `number` | 是 | 要检查的端口，必须大于 0 |
| `previousPid` | `number` | 是 | kill 前的 PID |
| `previousProcessName` | `string` | 是 | kill 前的进程名 |

### 返回类型

```ts
export type ReboundCheckResult = {
  port: number;
  occupied: boolean;
  rebound: boolean;
  sameProcessName: boolean;
  pid: number | null;
  processName: string | null;
  command: string | null;
  message: string | null;
};
```

| 字段 | 类型 | 说明 | UI 建议 |
| --- | --- | --- | --- |
| `port` | `number` | 被检查的端口 | 可用于提示文案 |
| `occupied` | `boolean` | 端口当前是否仍被占用 | `false` 表示端口已释放 |
| `rebound` | `boolean` | 当前占用者是否不是原 PID | `true` 时建议提示“端口被重新占用” |
| `sameProcessName` | `boolean` | 当前进程名是否与之前相同 | `true` 时可提示“可能被后台服务自动拉起” |
| `pid` | `number \| null` | 当前占用端口的 PID | 端口未占用时为 `null` |
| `processName` | `string \| null` | 当前占用进程名 | 端口未占用时为 `null` |
| `command` | `string \| null` | 当前占用进程完整命令 | 可放入详情或 tooltip |
| `message` | `string \| null` | 后端生成的中文提示 | 可直接用于 toast/message |

### 返回状态示例

#### 端口已释放

```ts
{
  port: 5173,
  occupied: false,
  rebound: false,
  sameProcessName: false,
  pid: null,
  processName: null,
  command: null,
  message: null,
}
```

#### 端口被同名进程重新监听

```ts
{
  port: 5173,
  occupied: true,
  rebound: true,
  sameProcessName: true,
  pid: 23456,
  processName: "node",
  command: "node /path/to/project/node_modules/.bin/vite",
  message: "端口 5173 已被重新监听，当前仍是“node”（PID 23456），可能由后台服务自动拉起",
}
```

#### 端口被其他进程占用

```ts
{
  port: 5173,
  occupied: true,
  rebound: true,
  sameProcessName: false,
  pid: 34567,
  processName: "python",
  command: "python -m http.server 5173",
  message: "端口 5173 已被重新占用，当前进程为“python”（PID 34567）",
}
```

### 失败情况

| 错误文本 | 触发条件 | UI 建议 |
| --- | --- | --- |
| `无效的端口` | `port == 0` | 不应出现，可视为开发错误 |
| 其他字符串 | `list_raw_ports` 扫描失败 | 建议静默忽略，避免打断 kill 成功主流程 |

## 前端调用流程建议

### 首次加载 / 手动刷新

```ts
const response = await invoke<PortListResponse>("list_ports");
items.value = response.items;
currentUser.value = response.currentUser;
```

### 结束进程

```ts
await invoke<KillResult>("kill_process", {
  pid: item.pid,
  force: false,
  expectedPort: item.port,
  expectedStartedAtTs: item.startedAtTs,
  expectedProcessName: item.processName,
  expectedUser: item.user,
});

await loadPorts({ silent: true });
```

### 强制结束进程

```ts
await invoke<KillResult>("kill_process", {
  pid: item.pid,
  force: true,
  expectedPort: item.port,
  expectedStartedAtTs: item.startedAtTs,
  expectedProcessName: item.processName,
  expectedUser: item.user,
});

await loadPorts({ silent: true });
```

### kill 后检查端口回弹

```ts
window.setTimeout(async () => {
  const result = await invoke<ReboundCheckResult>("check_port_rebound", {
    port: item.port,
    previousPid: item.pid,
    previousProcessName: item.processName,
  });

  if (result.rebound) {
    message.warning(result.message ?? `端口 ${item.port} 已被重新占用`);
    await loadPorts({ silent: true });
  }
}, 900);
```

## UI 开发注意事项

- 当前后端只支持 macOS。
- 当前只扫描 `TCP LISTEN`，不会返回 UDP 或已建立连接。
- 不要假设 `cwd` 和 `startedAt` 一定存在，权限或系统命令失败时可能为 `null`。
- 展示多个 `node`、`python`、`java` 等同名进程时，优先结合 `port`、`command`、`cwd` 区分。
- `user !== currentUser` 时，kill 可能因为权限不足失败，UI 可提前弱提示。
- `hostSummary` 适合表格展示；`hosts` 适合详情展示。
- `startedAtTs` 是排序辅助值，不建议直接展示给用户。
- 后端错误当前以中文为主，前端可直接展示，也可按关键字归类。
