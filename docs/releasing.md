# 发布清单

Port Kill 通过 GitHub Actions 构建 Apple Silicon 与 Intel 两种 macOS 发行版。发布工作流由 `.github/workflows/release.yml` 定义，并在推送 `v*` 标签时触发。

发布由 `pnpm release <version>`（`scripts/release.mjs`）驱动：它会校验状态、把 `CHANGELOG.md` 的 `Unreleased` 段落定版、同步三处版本号，并创建提交和标签。推送是手动的一步，标签推送后才会触发线上构建。

## 1. 平时维护 CHANGELOG

发布文案不在发布时才写。每完成一个用户可见的改动，就把条目追加到 `CHANGELOG.md` 顶部的 `## Unreleased` 段落（按 `性能与稳定性 / 安全性 / 体验` 等分类）。发布时 `Unreleased` 里应当已经是本次要发布的完整内容。

## 2. 发布前验证

安装锁定依赖并执行与本次变更相关的检查：

```bash
pnpm install --frozen-lockfile
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pnpm tauri build
```

桌面联调至少确认以下场景：

- 端口列表能正常加载，搜索与三种排序正常工作。
- 普通结束、强制结束、权限不足和列表过期提示符合预期。
- 同一进程监听多个端口时，影响范围提示完整。
- 端口被自动拉起时，回弹提示能够出现。
- 自动刷新失败时保留旧数据，手动刷新反馈正确。
- 深色、浅色主题及设置持久化正常。

## 3. 定版并发布

确认工作区干净（`Unreleased` 里的改动都已提交），然后先预览再发布。版本号遵循 SemVer，例如 `0.1.4`：

```bash
pnpm release 0.1.4 --dry-run   # 预览将要提取的 Release 说明，不改动任何文件
pnpm release 0.1.4             # 定版 CHANGELOG + 同步版本号 + 提交 + 打标签（本地）
```

`pnpm release 0.1.4` 会：

- 校验版本号合法、当前在 git 仓库内、工作区干净、`v0.1.4` 标签尚不存在、CHANGELOG 中没有该版本段落。
- 若 `## Unreleased` 为空（只有空行或小标题、没有实际条目），直接报错停下——避免推送标签后 CI 才发现没有 Release 说明。
- 把 `## Unreleased` 改写为 `## 0.1.4 - <当日日期>`，并在其上补一个新的空 `## Unreleased` 供下次使用。
- 通过 `scripts/bump-version.mjs` 把版本同时写入 `package.json`、`src-tauri/tauri.conf.json` 和 `src-tauri/Cargo.toml`。
- 创建 `chore(release): prepare v0.1.4` 提交，并打上带注释的 `v0.1.4` 标签。

确认无误后推送分支和标签以触发构建（脚本执行完会打印对应命令）：

```bash
git push origin <branch>
git push origin v0.1.4
```

也可以用 `pnpm release 0.1.4 --push` 在定版后直接推送。推送标签会立刻触发线上发布，请谨慎使用。

标签与三个项目版本不一致，或 CHANGELOG 中缺少对应版本标题时，构建会直接失败（这些在本地 `pnpm release` 阶段已被拦截）。标签推送后，在仓库的 Actions 页面确认两个架构任务都成功，再检查 GitHub Release 中的 `dmg` 与 `app.tar.gz` 产物。

## 4. 发布后检查

- 分别核对 Apple Silicon 与 Intel 产物名称和架构标识。
- 在干净环境安装并启动应用，确认端口扫描和进程操作可用。
- 检查 Release 正文是否来自正确的 CHANGELOG 版本段落。
- 若任一架构失败，不要复用同一标签覆盖发布；修复后递增补丁版本重新发布。
