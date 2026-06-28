# Kaubo 工具链整理 — 已收敛

> 阶段：✅ 已完成（Ops2 落地）
> 本文档记录了从散乱的 `Makefile.toml` + `cargo-make` 到统一 `kaubo-ops` 的收敛过程。

## 最终方案：Ops2

所有构建/测试/发布/部署入口统一为一个命令：

```bash
python kaubo-ops <cmd>
```

### 架构

```
kaubo-ops/
├── cli/main.py         ← 表示层：argparse + 命令路由
├── app/                 ← 应用层：用例编排（CI/Build/Test/Lint/Fmt/Dev/Bench/Release/Deploy）
├── domain/              ← 领域层：KauboProject 聚合根 + 值对象（WasmArtifact/RustWorkspace/…）
├── infra/               ← 基础设施层：CommandRunner/ProcessRunner/FileSystem/EventBus 抽象
└── config.json          ← 集中配置：路径映射、WASM 目标、工具版本要求
```

**依赖方向：表示层 → 应用层 → 领域层 ← 基础设施层**

### 全部命令（21 个）

| 命令 | 用途 |
|------|------|
| `ci` | 标准 CI（check + lint + fmt-check + test + build） |
| `ci-full` | CI + e2e |
| `check` | 快速类型检查（Rust + Web，无测试） |
| `build` | 构建所有产物（WASM + CLI + Web + VSCode） |
| `build-wasm` | 仅构建 WASM 双目标（web + nodejs） |
| `build-cli` | 仅构建 CLI 二进制（release） |
| `test` | 全部测试（Rust + Web + VSCode） |
| `test-rust` | Rust 测试 |
| `test-web` | Web 单元测试 |
| `test-web-e2e` | Web e2e 测试 |
| `test-vscode` | VSCode 扩展测试 |
| `lint` | 全部 lint（clippy + eslint） |
| `lint-rust` | Rust clippy |
| `lint-web` | Web eslint |
| `fmt` | 全部格式化（rustfmt + prettier，写入模式） |
| `fmt-check` | 全部格式检查（dry-run，不写入） |
| `dev` | 启动 Web 开发服务器（长驻进程，Ctrl-C 停止） |
| `release` | 发布到 GitHub Release |
| `deploy` | 部署到 nginx |
| `bench` | 运行跨语言性能对比 |
| `coverage` | 生成覆盖率报告 |

## 已解决的问题

| 旧问题 | 收敛后 |
|--------|--------|
| WASM 构建分裂（GUI vs VSCode 两次构建） | `build-wasm` 统一构建，两边消费同一产物 |
| 包管理器三元并存（cargo/pnpm/npm） | Ops2 统一编排，按需调用子工具 |
| Python ops 脚本深埋在 `next_kaubo/ops/` | 提升到 `kaubo-ops/`，单一发现点 |
| VSCode 是二等公民 | `test-vscode` / `build` 覆盖 VSCode |
| 入口工具需要额外安装 `cargo-make` | 零额外安装——`python kaubo-ops`，仅需 Python 3 + stdlib |
| `Makefile.toml` / `cargo make` 间接调用 | 已移除。直接 `python kaubo-ops` |

## 维护指引

- 改子项目路径 → 只改 `config.json`
- 加新命令 → `app/` 下写用例 + `cli/main.py` 注册路由
- 改构建逻辑 → 改对应的领域对象（`domain/rust.py`、`domain/gui.py` 等）
