# Kaubo 工具链整理方案

> 阶段：文档梳理（不做代码修改）
> 目标：分析当前 ops/构建入口的散乱问题，参考业界实践，给出收敛方案

## 1. 现状盘点：问题出在哪里

### 1.1 四层构建目标，三个包管理器，入口散落四处

```
packages/kaubo-features/
├── Makefile.toml              ← 顶层编排（cargo-make）
├── next_kaubo/                ← Rust workspace（cargo）
│   ├── Cargo.toml            13 个 crate + CLI binary
│   ├── crates/kaubo-wasm/    ← WASM 编译目标（wasm-pack）
│   ├── crates/kaubo-web-api/ ← Web/VSCode 共享 DTO
│   ├── gui/                   ← Web Playground（pnpm workspace）
│   │   └── packages/
│   │       ├── app/           SolidJS + Vite + CodeMirror
│   │       ├── wasm/          @kaubo/wasm（消费 kaubo-wasm 产物）
│   │       └── types/         共享 TS 类型
│   ├── kaubo2-cli/            ← CLI binary（cargo）
│   └── ops/                   ← Python 运维脚本
│       ├── benchmark/         domain/app/infra 分层
│       ├── deploy/            nginx 部署
│       ├── quality/           覆盖率
│       └── release/           GitHub Release 发布
└── vscode-extension/          ← VSCode 扩展（npm）
    ├── build-wasm.sh          独立的 WASM 构建脚本
    └── src/extension.js       diagnostics wiring
```

### 1.2 具体问题

| 问题 | 现状 | 影响 |
|------|------|------|
| **WASM 构建分裂** | GUI 的 WASM 产物在 `gui/packages/wasm/pkg/`（预生成提交），VSCode 的通过 `build-wasm.sh` 独立构建到 `vscode-extension/wasm/`。同一份 `kaubo-wasm` crate 被两次构建到不同目标目录。 | 两边可能版本不一致；新增 WASM 导出时必须手动同步两边；AI 不清楚该跑哪个 |
| **包管理器三元并存** | cargo（Rust）、pnpm（GUI）、npm（VSCode）。`Makefile.toml` 分别 `cd` 到不同目录执行不同命令 | 顶层 `cargo make` 只是薄封装，排查失败要追溯到子目录 |
| **Python ops 脚本深埋** | 覆盖率/benchmark/发布/部署脚本在 `next_kaubo/ops/`，通过 `cargo make ops-*` 间接调用 | 新人（包括 AI）很难发现这些能力；脚本参数只能在 `Makefile.toml` 里看 |
| **VSCode 是二等公民** | 扩展目录在 workspace 根外，`build-wasm.sh` 不经过 `cargo make`，`Makefile.toml` 里只有 test/package 两个任务 | CI 不会自动验证 VSCode 的 WASM 能否构建 |
| **跨目标依赖不可见** | `kaubo-wasm` → GUI + VSCode 的消费关系没有在任何地方声明，纯粹靠约定和文档 | 改 `kaubo-wasm` 的 API 后，只跑 `cargo make ci` 不会重建 GUI/VSCode 的 WASM |
| **入口工具需要额外安装** | `cargo make` 需要 `cargo install cargo-make`，不属于 Rust 标准工具链 | 新人 clone 后无法立刻 `./something ci` |

### 1.3 当前事实上有多少个"入口"

开发者需要知道的所有入口：

```
# Rust
cd next_kaubo && cargo check/test/build/clippy

# WASM (GUI)
cd next_kaubo && wasm-pack build crates/kaubo-wasm --target web --out-dir gui/packages/wasm/pkg

# WASM (VSCode)
cd vscode-extension && bash build-wasm.sh

# Web GUI
cd next_kaubo/gui && pnpm dev/test/build

# VSCode
cd vscode-extension && npm test / npm run package

# Ops
cd next_kaubo && python3 ops/quality/coverage.py
cd next_kaubo && python3 ops/benchmark/runner.py bench --release
cd next_kaubo && python3 ops/release/publish.py --bump patch

# 顶层（需要 cargo-make）
cargo make ci
```

**共 7 种入口模式，分布在 4 个不同目录。**

---

## 2. 业界实践调查

### 2.1 模式对比

| 模式 | 代表项目 | 入口形式 | 学习成本 | AI 友好度 | 适合场景 |
|------|---------|---------|---------|----------|---------|
| **xtask** | rust-analyzer, Helix, turbo, rye | `cargo xtask <cmd>` (Rust 写的任务运行器) | ⭐⭐ 需编译 | ⭐⭐⭐⭐⭐ 类型安全、可 grep | Rust-heavy，多目标 |
| **justfile** | 很多 Rust 中型项目 | `just <cmd>` | ⭐ 极低 | ⭐⭐⭐⭐ 语法简单 | 轻量替代 Make |
| **scripts/ + Makefile** | Zed, Bevy, Deno | `./scripts/ci.sh` | ⭐ 极低 | ⭐⭐⭐⭐⭐ 纯 shell，所见即所得 | 跨语言、目标多 |
| **cargo-make** | 当前 kaubo | `cargo make <cmd>` | ⭐⭐ 需安装 | ⭐⭐⭐ TOML 配置 | 已有投入 |
| **taskfile** | Go 生态 | `task <cmd>` | ⭐ 极低 | ⭐⭐⭐⭐ YAML | 跨语言 |
| **npm scripts / pnpm** | Web 前端项目 | `pnpm run <cmd>` | ⭐ 极低 | ⭐⭐⭐⭐ | 纯 JS/TS 项目 |

### 2.2 重点案例

#### rust-analyzer（xtask 模式）

```
rust-analyzer/
├── xtask/           ← 独立 Rust crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # cargo xtask
│       ├── dist.rs          # cargo xtask dist
│       ├── install.rs       # cargo xtask install
│       └── fuzz_tests.rs    # cargo xtask fuzz
├── Cargo.toml       ← workspace 包含 xtask
└── .cargo/config.toml  ← alias: xtask = "run --package xtask --"
```

**优点**：
- 只需 Rust 工具链（项目本来就依赖 Rust）
- 类型安全，IDE 支持好
- `cargo xtask --help` 自动发现所有子命令
- AI 可以直接 `grep "fn.*xtask"` 找到所有任务

**缺点**：
- 首次运行需要编译 xtask（~10s），后续增量编译很快
- 不适合纯 shell 操作（要写 Rust 调用 Command）

#### Zed（scripts/ 模式）

```
zed/
├── script/
│   ├── bootstrap      # 首次设置
│   ├── build          # 构建
│   ├── test           # 测试
│   ├── lint           # Lint
│   ├── format         # 格式化
│   └── ...            # 每个任务一个脚本
```

**优点**：
- 零额外依赖
- 每个脚本独立可执行、可理解
- AI 直接 `ls script/` 就知道所有入口
- 跨语言无摩擦

**缺点**：
- Windows 兼容需要 `.ps1` / `.sh` 双份（或要求 Git Bash）
- 共享逻辑需要 source 或重复

#### Deno（混合：Python tools/ + cargo）

```
deno/
├── tools/
│   ├── build.py
│   ├── lint.py
│   ├── format.py
│   └── ...
├── Cargo.toml
└── Makefile         # 薄封装，委托给 tools/
```

**优点**：
- Python 跨平台统一
- 复杂逻辑比 shell 好维护
- 已有的 Python ops 脚本可以自然迁移

---

## 3. 入口收敛方案（第一步）→ 最终形态见第 8 节

> 本节是初始方案——用 `tool/` 目录收敛入口。最终采用 DDD 架构的 Ops2，详见[第 8 节](#8-ops2ddd-架构细化)。

### 3.1 初步思路：shell 脚本收敛

**核心原则：一个目录，一个入口模式，AI 一眼看到全貌。**

具体理由：

1. **Kaubo 已经有 Python ops 脚本**（benchmark 甚至还做了 domain/app/infra 分层）。放弃 Python 改用 xtask 会增加迁移成本，而且 benchmark 这类数据分析任务用 Python 天然更合适。

2. **Shell 脚本做薄封装**：每个任务一个脚本，脚本内部调用 `cargo`/`pnpm`/`npm`/`python3`。AI 能直接读脚本理解完整流程。

3. **保留 `Makefile.toml` 做 CI 别名**：不删除，但让它委托给 `tool/` 脚本（而不是反过来，现在是 Makefile 直接执行命令，脚本藏在 ops/ 深处）。

4. **WASM 构建收口**：不再有两套 WASM 构建路径。一个脚本同时产出 GUI 和 VSCode 需要的 WASM 产物。

### 3.2 目标目录结构

```
packages/kaubo-features/
├── tool/                          ← ★ 统一入口（新增）
│   ├── README.md                  # 每个脚本的用途说明
│   ├── ci                         # 标准 CI（= check + test + lint）
│   ├── ci-full                    # CI + e2e
│   ├── check                      # 快速类型检查（Rust + Web）
│   ├── build                      # 构建所有产物（CLI + WASM + Web + VSCode）
│   ├── build-wasm                 # ★ WASM 统一构建入口
│   ├── build-cli                  # 只构建 CLI binary
│   ├── test                       # 全部测试
│   ├── test-rust                  # Rust 测试
│   ├── test-web                   # Web 单元测试
│   ├── test-web-e2e               # Web e2e
│   ├── test-vscode                # VSCode 测试
│   ├── lint                       # 全部 lint
│   ├── lint-rust                  # clippy
│   ├── lint-web                   # eslint
│   ├── fmt                        # 全部格式化
│   ├── fmt-rust                   # rustfmt
│   ├── fmt-web                    # prettier
│   ├── coverage                   # 覆盖率
│   ├── benchmark                  # 性能测试
│   ├── release                    # 发布
│   ├── deploy                     # 部署
│   └── _lib/                      # 共享 helper（可选）
│       └── utils.sh
├── next_kaubo/                    # Rust workspace（不变）
│   ├── crates/
│   ├── gui/
│   ├── kaubo2-cli/
│   └── ops/                       # Python 脚本（保留，被 tool/ 调用）
├── vscode-extension/              # VSCode 扩展（不变）
├── Makefile.toml                  # 保留，委托给 tool/
│                                  # e.g. args = ["tool/ci"]
└── AGENTS.md                      # 更新：指向 tool/ 而非 Makefile.toml
```

### 3.3 关键设计决策

#### WASM 构建收口（最重要的收敛）

```
tool/build-wasm
    ├── wasm-pack build crates/kaubo-wasm --target web
    │   └── → gui/packages/wasm/pkg/       （Web 消费）
    └── wasm-pack build crates/kaubo-wasm --target nodejs
        └── → vscode-extension/wasm/        （VSCode 消费）
```

一次脚本，两个目标，产物各自落到消费者目录。`tool/build` 和 `tool/ci` 依赖 `tool/build-wasm`。

#### 每个脚本自描述

```bash
#!/usr/bin/env bash
# tool/ci — 标准 CI 检查
#
# 运行: ./tool/ci
# 包含: Rust check + clippy + fmt + test + Web test + build + VSCode test
# 不包含: e2e (用 tool/ci-full)
#
# 前置条件: Rust, pnpm, npm, wasm-pack
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

"$SCRIPT_DIR/check"
"$SCRIPT_DIR/test"
"$SCRIPT_DIR/lint"
```

#### AI 友好的设计原则

1. **单一发现点**：`ls tool/` 就知道所有任务
2. **自描述头注释**：每个脚本前 10 行说明用途、参数、前置条件
3. **可独立运行**：`./tool/test-rust` 不依赖顶层状态
4. **命名即意图**：动词（build/test/lint/fmt）+ 可选目标（rust/web/wasm/vscode）
5. **幂等**：重复运行不产生副作用
6. **错误清晰**：`set -euo pipefail`，失败立刻停止并显示行号

---

## 4. 迁移路径（旧方案 → 已更新为第 8.11 节）

> 最终迁移方案见 [8.11 迁移兼容性](#811-迁移兼容性)，包含四阶段共存收敛计划。

### Phase A（当前 — 文档确认）

- [x] 现状盘点
- [ ] 方案评审
- [ ] 确定 `tool/` 目录的完整任务清单

### Phase B（创建 tool/ 目录，不删旧入口）

- [ ] 创建 `tool/` 目录和全部脚本
- [ ] 每个脚本调用现有命令（cargo/pnpm/npm/python3）
- [ ] `tool/build-wasm` 收口两处 WASM 构建
- [ ] 脚本全部加 `set -euo pipefail` 和头注释
- [ ] 验证 `./tool/ci` 和 `cargo make ci` 行为一致
- [ ] 更新 `AGENTS.md`：推荐 `tool/` 为首选入口

### Phase C（更新 Makefile.toml 委托）

- [ ] `Makefile.toml` 任务改为调用 `tool/` 脚本
- [ ] 验证所有 `cargo make *` 任务仍然工作
- [ ] 删除 `vscode-extension/build-wasm.sh`（功能已被 `tool/build-wasm` 覆盖）

### Phase D（清理旧路径，可选）

- [ ] 评估是否保留 `cargo-make` 依赖
- [ ] 如果不需要，添加 `.cargo/config.toml` alias 作为替代
- [ ] 更新所有文档链接

---

## 5. 关于 Skill 的考虑

### 5.1 当前 AGENTS.md 已经很好

`packages/kaubo-features/AGENTS.md` 已经包含了：
- 构建工具说明（当前入口 `cargo make`）
- 分层约束
- 测试策略
- 仓库卫生规则

### 5.2 Ops2 落地后的 Skill 关键指令

Ops2 落地后，Skill 的核心约束点会变成：

1. **唯一入口**：`python kaubo-ops <cmd>`，不再有 `cargo make`/`pnpm build`/`bash build-wasm.sh`
2. **构建链路**：改 Rust → `python kaubo-ops test`（自动只跑 Rust），改 WASM API → `python kaubo-ops build-wasm && python kaubo-ops test`（自动覆盖 Web + VSCode）
3. **子项目规则**：`package.json` 无 scripts → AI 在子项目中找不到可添加命令的位置，自然被引导到 `kaubo-ops/app/` 下新建用例
4. **分层约束速查**：改领域层 → 所有用例自动受益；改基础设施 → 只影响命令执行方式；改应用层 → 只影响编排顺序
5. **常见失败排查**：WASM 不匹配 → `python kaubo-ops build-wasm`；GUI 白屏 → 检查 WASM 产物是否在聚合根指定的路径下

### 5.3 Skill 的物理约束价值

Ops2 的硬性规则（无 scripts、单向调用）是物理层面的防错机制——不是"AI 应该遵守"，而是"AI 想违反也违反不了"。Skill 文档只需描述"怎么用"，不需要描述"不要做什么"，因为做错事的入口已经被物理封死了。

---

## 6. 硬性规则：让子项目"变哑"

Ops2 的根基性设计决策——不是简单的"把脚本挪个地方"，而是**从根本上改变子项目的角色**。

### 6.1 核心约束（物理层面防 AI 乱写）

| # | 规则 | 目的 |
|---|------|------|
| ① | **子项目 `package.json` 禁止出现 `scripts` 字段** | 没有 `"dev"`、`"build"`、`"test"`，彻底封死 AI 写入脚本的入口 |
| ② | **子项目禁止任何生命周期钩子** | 无 `preinstall`、`postinstall`、`prepare`，杜绝循环调用和隐式行为 |
| ③ | **调用方向单向：Ops2 → 子项目** | 子项目绝不反向引用 Ops2（连相对路径 `../kaubo-ops` 都不存在），循环依赖在物理上被消灭 |
| ④ | **Ops2 内部执行裸命令，不经过子项目的包管理器脚本** | 构建 GUI 时直接执行 `vite build`（或 `pnpm exec vite build`），绝不执行 `pnpm run build`，避免递归 |

### 6.2 子项目的"哑"形态

改造前（GUI `package.json`）：
```json
{
  "name": "@kaubo/app",
  "scripts": {
    "dev": "vite",
    "build": "pnpm --filter @kaubo/types build && tsc --noEmit && vite build",
    "test": "vitest run",
    "lint": "eslint src/",
    "format": "prettier --write 'src/**/*.{ts,tsx,css}'"
  }
}
```

改造后：
```json
{
  "name": "@kaubo/app",
  "dependencies": { ... },
  "devDependencies": { ... }
}
```

所有行为定义上移到 Ops2 的领域层：

```python
@dataclass
class GuiApp:
    root: Path
    wasm_pkg: Path

    def dev_command(self) -> list[str]:
        return ["pnpm", "exec", "vite"]

    def build_command(self) -> list[str]:
        return ["pnpm", "exec", "vite", "build"]

    def test_command(self) -> list[str]:
        return ["pnpm", "exec", "vitest", "run"]

    def lint_command(self) -> list[str]:
        return ["pnpm", "exec", "eslint", "src/"]

    def fmt_command(self) -> list[str]:
        return ["pnpm", "exec", "prettier", "--write", "src/**/*.{ts,tsx,css}"]
```

### 6.3 为什么物理约束比文档约束强

文档说"请勿在子项目中添加构建脚本"——AI 可能忽略，可能忘了，可能在一个很长 conversation 的中途被上下文覆盖。

`package.json` 里没有 `scripts` 字段——AI 想写也写不进去，因为它在子项目里找不到"在哪里添加命令"的先例。**物理空白 > 文档约束**。

---

## 7. 总结

| 维度 | 现状 | 目标 |
|------|------|------|
| 入口数量 | 7 种，散落 4 个目录 | 唯一入口 `python kaubo-ops <cmd>` |
| 子项目角色 | 各自有 scripts/build 逻辑 | 纯静态资源，仅含源码和依赖清单 |
| WASM 构建 | 两套独立脚本 | 领域对象 `WasmArtifact` 一次构建多目标 |
| 跨目标依赖 | 隐性约定 | 聚合根 `KauboProject` 显式持有所有消费关系 |
| AI 防错能力 | 依赖 AGENTS.md 文档约束 | `package.json` 物理无 scripts + 单向调用 |
| 新人上手 | `cargo make` 需额外安装 | `python kaubo-ops ci`（只需 Python stdlib） |
| Python ops 可见性 | 深埋在 `next_kaubo/ops/` | 收归到 `kaubo-ops/` 领域层 |

**核心原则：将所有构建、测试、部署等行为从子项目中剥离，统一收归到根目录的 Ops2 编排层。子项目退化为纯静态资源，Ops2 成为唯一的智能中枢。**

---

## 8. Ops2：DDD 架构细化

> 前文第 3 节给出了入口收敛的目标，第 6 节定义了子项目"变哑"的硬性规则。本节给出完整实现方案：用 DDD 四层架构，把"构建过程"建模为一个领域。

### 8.1 设计哲学

Ops2 的核心命题不是"写一堆构建脚本"，而是**把工程流程写成可执行的领域模型**。代码读起来应该像一份"可执行的工程手册"——应用层描述"做什么"，领域层定义"是什么"，基础设施层负责"怎么做"。

为什么个人项目适合 DDD：

1. **六年后秒懂**：领域对象直接映射工程概念（`WasmArtifact`、`VscodeExtension`），不需要回忆任何 Shell 魔法。看应用层用例流程就能重建完整认知。
2. **AI 友好度拉满**：显式分层 + 类型提示 = AI 能精确推理边界。加新命令时 AI 能直接告诉你"在应用层新建用例，在领域层扩展聚合根，在基础设施层添加执行方法"，而不是在几百行 Bash 里猜应该改哪一行。
3. **试错成本极低**：个人项目，大胆重构领域模型，没有任何团队依赖。

### 8.2 四层职责切割

```
┌─────────────────────────────────────────────────┐
│  表示层 / CLI                                    │
│  解析用户输入，路由到用例，打印进度/错误           │
│  "用户想干什么"                                   │
├─────────────────────────────────────────────────┤
│  应用层 / Use Cases                              │
│  编排流程顺序和依赖关系，不含任何命令执行细节       │
│  "先做什么，再做什么"                              │
├─────────────────────────────────────────────────┤
│  领域层 / Domain                                 │
│  工程中的"事物"及其规则——带行为的对象，非路径字符串  │
│  "这是什么，它应该满足什么规则"                     │
├─────────────────────────────────────────────────┤
│  基础设施层 / Infrastructure                      │
│  真正调用 subprocess、操作文件系统、发 HTTP 请求    │
│  "怎么执行"                                       │
└─────────────────────────────────────────────────┘
```

依赖方向：**表示层 → 应用层 → 领域层 ← 基础设施层**（领域层不依赖任何外层，基础设施层实现领域层定义的抽象接口）。

### 8.3 领域层：核心模型

#### 聚合根：`KauboProject`

整个 Kaubo 项目是一个聚合根，持有所有子组件的引用。**所有路径映射和制品消费关系只在这一处维护**——改目录结构只改这里，所有用例自动生效。

```python
from dataclasses import dataclass, field
from pathlib import Path

@dataclass
class KauboProject:
    """聚合根：Kaubo 项目的完整工程模型"""
    root: Path

    # 子项目路径（唯一的路径定义点）
    rust_workspace: Path = field(init=False)
    cli_crate: Path = field(init=False)
    wasm_crate: Path = field(init=False)
    gui_root: Path = field(init=False)
    gui_wasm_pkg: Path = field(init=False)
    vscode_root: Path = field(init=False)
    vscode_wasm_dir: Path = field(init=False)

    def __post_init__(self):
        nk = self.root / "next_kaubo"
        self.rust_workspace = nk
        self.cli_crate = nk / "kaubo2-cli"
        self.wasm_crate = nk / "crates" / "kaubo-wasm"
        self.gui_root = nk / "gui"
        self.gui_wasm_pkg = self.gui_root / "packages" / "wasm" / "pkg"
        self.vscode_root = self.root / "vscode-extension"
        self.vscode_wasm_dir = self.vscode_root / "wasm"

    # ---- 工厂方法：创建领域对象 ----

    def create_wasm_artifacts(self) -> list["WasmArtifact"]:
        """WASM 构建的两个目标产物"""
        return [
            WasmArtifact(
                crate=self.wasm_crate,
                target=WasmTarget.WEB,
                output_dir=self.gui_wasm_pkg,
                consumer="GUI Playground",
            ),
            WasmArtifact(
                crate=self.wasm_crate,
                target=WasmTarget.NODEJS,
                output_dir=self.vscode_wasm_dir,
                consumer="VSCode Extension",
            ),
        ]

    def create_rust_workspace(self) -> "RustWorkspace":
        return RustWorkspace(root=self.rust_workspace)

    def create_gui_app(self) -> "GuiApp":
        return GuiApp(root=self.gui_root, wasm_pkg=self.gui_wasm_pkg)

    def create_vscode_extension(self) -> "VscodeExtension":
        return VscodeExtension(root=self.vscode_root, wasm_dir=self.vscode_wasm_dir)
```

#### 值对象：有行为的"事物"

```python
from enum import Enum
from abc import ABC, abstractmethod

class WasmTarget(Enum):
    WEB = "web"
    NODEJS = "nodejs"

@dataclass
class WasmArtifact:
    """WASM 构建产物——知道自己怎么构建、产到哪里、被谁消费"""
    crate: Path
    target: WasmTarget
    output_dir: Path
    consumer: str  # 人类可读的消费者名称，用于日志

    @property
    def out_name(self) -> str:
        return "kaubo_wasm"

    def build_command(self) -> list[str]:
        return [
            "wasm-pack", "build", str(self.crate),
            "--target", self.target.value,
            "--out-dir", str(self.output_dir),
            "--out-name", self.out_name,
        ]

@dataclass
class RustWorkspace:
    """Rust workspace——知道自己包含哪些 crate、怎么检查/测试/构建"""
    root: Path

    def check_command(self) -> list[str]:
        return ["cargo", "check", "--workspace", "--all-targets"]

    def test_command(self) -> list[str]:
        return ["cargo", "test", "--workspace"]

    def clippy_command(self) -> list[str]:
        return ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]

    def fmt_check_command(self) -> list[str]:
        return ["cargo", "fmt", "--all", "--", "--check"]

    def build_release_command(self, package: str) -> list[str]:
        return ["cargo", "build", "--release", "-p", package]

@dataclass
class GuiApp:
    """Web Playground——纯静态资源，行为全部由 Ops2 定义。

    对应的 package.json 无 scripts 字段。
    Ops2 直接执行裸命令（pnpm exec vite），不经过子项目包管理器脚本。
    """
    root: Path
    wasm_pkg: Path  # WASM 产物消费位置

    def dev_command(self) -> list[str]:
        """启动开发服务器（Ops2 托管进程，透传信号）"""
        return ["pnpm", "exec", "vite"]

    def build_command(self) -> list[str]:
        return ["pnpm", "exec", "vite", "build"]

    def test_command(self) -> list[str]:
        return ["pnpm", "exec", "vitest", "run"]

    def lint_command(self) -> list[str]:
        return ["pnpm", "exec", "eslint", "src/"]

    def fmt_check_command(self) -> list[str]:
        return ["pnpm", "exec", "prettier", "--check", "src/**/*.{ts,tsx,css}"]

    def fmt_command(self) -> list[str]:
        return ["pnpm", "exec", "prettier", "--write", "src/**/*.{ts,tsx,css}"]

    def types_build_command(self) -> list[str]:
        """构建共享类型包（@kaubo/types）"""
        return ["pnpm", "exec", "tsc", "-p", "packages/types/tsconfig.json"]


@dataclass
class VscodeExtension:
    """VSCode 扩展——纯静态资源，行为全部由 Ops2 定义。

    对应的 package.json 无 scripts 字段。
    """
    root: Path
    wasm_dir: Path  # WASM 产物消费位置

    def test_command(self) -> list[str]:
        return ["node", "--test", "tests/*.test.js"]

    def package_command(self) -> list[str]:
        return ["pnpm", "exec", "vsce", "package"]

@dataclass
class ReleaseVersion:
    """发布版本——知道 semver 规则和 bump 逻辑"""
    major: int
    minor: int
    patch: int

    @classmethod
    def parse(cls, s: str) -> "ReleaseVersion":
        parts = s.strip().split(".")
        if len(parts) != 3:
            raise ValueError(f"版本号格式错误: {s} (需要 X.Y.Z)")
        return cls(int(parts[0]), int(parts[1]), int(parts[2]))

    def bump(self, level: str) -> "ReleaseVersion":
        if level == "major":
            return ReleaseVersion(self.major + 1, 0, 0)
        elif level == "minor":
            return ReleaseVersion(self.major, self.minor + 1, 0)
        else:
            return ReleaseVersion(self.major, self.minor, self.patch + 1)

    def __str__(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"
```

### 8.4 基础设施层：抽象接口 + 具体实现

```python
from abc import ABC, abstractmethod
from dataclasses import dataclass
import subprocess
import sys
from pathlib import Path

# ── 抽象接口 ──────────────────────────────────────

@dataclass
class CommandResult:
    exit_code: int
    stdout: str
    stderr: str

    @property
    def ok(self) -> bool:
        return self.exit_code == 0

class CommandRunner(ABC):
    """执行外部命令的抽象——唯一的外部副作用入口"""
    @abstractmethod
    def run(self, cmd: list[str], cwd: Path | None = None,
            env: dict[str, str] | None = None) -> CommandResult:
        ...

class FileSystem(ABC):
    """文件系统操作抽象"""
    @abstractmethod
    def exists(self, path: Path) -> bool: ...
    @abstractmethod
    def read_text(self, path: Path) -> str: ...
    @abstractmethod
    def write_text(self, path: Path, content: str) -> None: ...
    @abstractmethod
    def mkdir_p(self, path: Path) -> None: ...
    @abstractmethod
    def rmtree(self, path: Path) -> None: ...

class EventBus(ABC):
    """进度/事件输出抽象—— CLI 输出、CI 日志、结构化文件都通过它"""
    @abstractmethod
    def emit(self, level: str, message: str) -> None: ...

# ── 具体实现 ──────────────────────────────────────

class RealCommandRunner(CommandRunner):
    def run(self, cmd: list[str], cwd: Path | None = None,
            env: dict[str, str] | None = None) -> CommandResult:
        result = subprocess.run(cmd, cwd=cwd, env=env,
                                capture_output=True, text=True)
        return CommandResult(
            exit_code=result.returncode,
            stdout=result.stdout,
            stderr=result.stderr,
        )

class RealFileSystem(FileSystem):
    def exists(self, path: Path) -> bool: return path.exists()
    def read_text(self, path: Path) -> str: return path.read_text()
    def write_text(self, path: Path, content: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    def mkdir_p(self, path: Path) -> None: path.mkdir(parents=True, exist_ok=True)
    def rmtree(self, path: Path) -> None:
        import shutil
        if path.exists():
            shutil.rmtree(path)

class ConsoleEventBus(EventBus):
    def emit(self, level: str, message: str) -> None:
        prefix = {"info": "  →", "error": "  ✗", "success": "  ✓", "step": "\n[step]"}.get(level, "   ")
        print(f"{prefix} {message}", file=sys.stderr if level == "error" else sys.stdout)

# ── 长驻进程支持 ────────────────────────────────

class ProcessHandle(ABC):
    """管理一个已启动的子进程生命周期。"""

    @abstractmethod
    def wait(self) -> int:
        """阻塞等待进程结束，返回 exit code。"""
        ...

    @abstractmethod
    def terminate(self) -> None:
        """发送 SIGTERM / TerminateProcess。"""
        ...

    @abstractmethod
    def kill(self) -> None:
        """发送 SIGKILL / TerminateProcess(force)。"""
        ...

    @abstractmethod
    @property
    def pid(self) -> int:
        """操作系统 PID。"""
        ...


class ProcessRunner(ABC):
    """启动长驻进程的抽象——与 CommandRunner 互补。

    CommandRunner.run() = 一次性执行，等 exit code。
    ProcessRunner.spawn() = 启动后立刻返回，返回 ProcessHandle 托管生命周期。
    """

    @abstractmethod
    def spawn(self, cmd: list[str], cwd: Path | None = None,
              env: dict[str, str] | None = None) -> ProcessHandle:
        ...


class RealProcessRunner(ProcessRunner):
    def spawn(self, cmd: list[str], cwd: Path | None = None,
              env: dict[str, str] | None = None) -> ProcessHandle:
        p = subprocess.Popen(cmd, cwd=cwd, env=env)
        return _RealProcessHandle(p)


class _RealProcessHandle(ProcessHandle):
    def __init__(self, popen: subprocess.Popen):
        self._popen = popen

    def wait(self) -> int:
        return self._popen.wait()

    def terminate(self) -> None:
        self._popen.terminate()

    def kill(self) -> None:
        self._popen.kill()

    @property
    def pid(self) -> int:
        return self._popen.pid
```

### 8.5 应用层：用例编排

应用层只持有基础设施抽象和领域对象，**不含任何 `subprocess.run` 或路径硬编码**。

```python
class CiPipeline:
    """标准 CI 用例：check + test + lint，全目标"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "CI Pipeline")

        # 1. 环境检查
        if not self._check_tools(["cargo", "pnpm", "npm", "wasm-pack"]):
            return False

        # 2. Rust 检查
        self.events.emit("step", "Rust check")
        r = self.runner.run(
            project.create_rust_workspace().check_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Rust check failed:\n{r.stderr}")
            return False

        # 3. Rust lint
        self.events.emit("step", "Rust clippy")
        r = self.runner.run(
            project.create_rust_workspace().clippy_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Clippy failed:\n{r.stderr}")
            return False

        # 4. Rust fmt
        self.events.emit("step", "Rust fmt check")
        r = self.runner.run(
            project.create_rust_workspace().fmt_check_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", "Formatting check failed")
            return False

        # 5. Rust test
        self.events.emit("step", "Rust test")
        r = self.runner.run(
            project.create_rust_workspace().test_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Rust test failed:\n{r.stderr}")
            return False

        # 6. WASM
        if not BuildWasm(self.runner, self.fs, self.events).run(project):
            return False

        # 7. Web test
        self.events.emit("step", "Web test")
        gui = project.create_gui_app()
        r = self.runner.run(gui.test_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"Web test failed:\n{r.stderr}")
            return False

        # 8. Web build
        self.events.emit("step", "Web build")
        # types 包先构建
        self.runner.run(gui.types_build_command(), cwd=gui.root)
        r = self.runner.run(gui.build_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"Web build failed:\n{r.stderr}")
            return False

        # 9. VSCode test
        self.events.emit("step", "VSCode test")
        vscode = project.create_vscode_extension()
        r = self.runner.run(vscode.test_command(), cwd=vscode.root)
        if not r.ok:
            self.events.emit("error", f"VSCode test failed:\n{r.stderr}")
            return False

        self.events.emit("success", "CI passed")
        return True

    def _check_tools(self, tools: list[str]) -> bool:
        import shutil
        for t in tools:
            if shutil.which(t) is None:
                self.events.emit("error", f"Missing tool: {t}")
                return False
        return True


class BuildWasm:
    """构建 WASM 用例——一次构建，多目标产出"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Building WASM")
        for artifact in project.create_wasm_artifacts():
            self.events.emit("info", f"  {artifact.target.value} → {artifact.consumer}")
            r = self.runner.run(
                artifact.build_command(),
                cwd=project.rust_workspace,
            )
            if not r.ok:
                self.events.emit("error", f"WASM ({artifact.target.value}) failed:\n{r.stderr}")
                return False
        self.events.emit("success", "WASM built for all targets")
        return True


class DevServer:
    """启动 Web 开发服务器——长驻进程，透传 Ctrl-C 信号。

    与 CiPipeline/BuildWasm 的关键区别：
    - 它们用 CommandRunner.run()（同步等退出，适合一次性命令）
    - DevServer 用 ProcessRunner.spawn()（启动后立刻返回 ProcessHandle）
    """

    def __init__(self, proc_runner: ProcessRunner, events: EventBus):
        self.proc_runner = proc_runner
        self.events = events

    def run(self, project: KauboProject) -> int:
        gui = project.create_gui_app()
        cmd = gui.dev_command()

        self.events.emit("info", f"Starting: {' '.join(cmd)}")
        self.events.emit("info", f"  cwd: {gui.root}")
        self.events.emit("info", "  Press Ctrl-C to stop")

        handle = self.proc_runner.spawn(cmd, cwd=gui.root)

        # 阻塞等进程结束（或被 Ctrl-C 中断）
        import signal
        def _forward_signal(signum, frame):
            self.events.emit("info", f"Received signal {signum}, forwarding to pid={handle.pid}")
            handle.terminate()

        original_sigint = signal.signal(signal.SIGINT, _forward_signal)
        original_sigterm = signal.signal(signal.SIGTERM, _forward_signal)
        try:
            exit_code = handle.wait()
            return exit_code
        finally:
            signal.signal(signal.SIGINT, original_sigint)
            signal.signal(signal.SIGTERM, original_sigterm)
```

`CommandRunner.run()` vs `ProcessRunner.spawn()`——两类命令，两种执行模式：
- **一次性命令**（check/test/build/lint）：`CommandRunner.run()` → 等 exit code → 返回 `CommandResult`
- **长驻进程**（dev server）：`ProcessRunner.spawn()` → 立刻返回 `ProcessHandle` → 托管生命周期 + 透传信号

表示层在路由时根据命令类型注入不同的 runner。`DevServer` 用例依赖 `ProcessRunner`，`CiPipeline` 等依赖 `CommandRunner`，互不污染。

注意 `CiPipeline.run()` 的可读性——它就是 Kaubo CI 的"可执行文档"，流程顺序一目了然。没有任何 `subprocess.run` 或路径拼接——它们分别属于基础设施层和领域层。

### 8.6 表示层：CLI 入口

表示层只做三件事：解析参数、路由到用例、打印结果。唯一的用户入口是 `python kaubo-ops <cmd>`。

```python
# kaubo-ops/__main__.py —— 使得 `python kaubo-ops` 可用
import sys
from pathlib import Path

# __main__.py 在 kaubo-ops/ 下，把自己加到 sys.path 后即可用 flat import
sys.path.insert(0, str(Path(__file__).resolve().parent))

from cli.main import main

if __name__ == "__main__":
    sys.exit(main())
```

```python
# kaubo-ops/cli/main.py
import argparse
import sys
from pathlib import Path
from domain.project import KauboProject
from app.ci_pipeline import CiPipeline
from app.build_wasm import BuildWasm
from app.dev_server import DevServer
from app.lint_all import LintAll
from app.publish_release import PublishRelease
from infra.command import RealCommandRunner
from infra.filesystem import RealFileSystem
from infra.events import ConsoleEventBus
from infra.process import RealProcessRunner

def main() -> int:
    parser = argparse.ArgumentParser(description="Kaubo Ops2 — 领域驱动的工程编排系统")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("ci",       help="标准 CI（check + lint + fmt + test + build）")
    sub.add_parser("ci-full",  help="CI + e2e")
    sub.add_parser("check",    help="快速类型检查（Rust + Web）")
    sub.add_parser("build",    help="构建所有产物（CLI + WASM + Web + VSCode）")
    sub.add_parser("build-wasm", help="仅构建 WASM 双目标")
    sub.add_parser("test",     help="全部测试")
    sub.add_parser("lint",     help="全部 lint（clippy + eslint）")
    sub.add_parser("fmt",      help="全部格式化（rustfmt + prettier）")
    sub.add_parser("dev",      help="启动 Web 开发服务器")
    sub.add_parser("release",  help="发布到 GitHub Release")
    sub.add_parser("deploy",   help="部署到 nginx")
    sub.add_parser("bench",    help="运行 benchmark")
    sub.add_parser("coverage", help="生成覆盖率报告")

    # release 子参数
    # ... (略)

    args = parser.parse_args()

    # 组装依赖（依赖注入）
    runner = RealCommandRunner()
    proc_runner = RealProcessRunner()
    fs = RealFileSystem()
    events = ConsoleEventBus()
    project = KauboProject(root=Path(__file__).resolve().parents[2])  # kaubo-ops/cli/ → kaubo-ops/ → root

    # 路由
    routes = {
        "ci":         lambda: CiPipeline(runner, fs, events).run(project),
        "ci-full":    lambda: CiFullPipeline(runner, fs, events).run(project),
        "check":      lambda: QuickCheck(runner, fs, events).run(project),
        "build":      lambda: BuildAll(runner, fs, events).run(project),
        "build-wasm": lambda: BuildWasm(runner, fs, events).run(project),
        "test":       lambda: TestAll(runner, fs, events).run(project),
        "lint":       lambda: LintAll(runner, fs, events).run(project),
        "fmt":        lambda: FmtAll(runner, fs, events).run(project),
        "dev":        lambda: DevServer(proc_runner, events).run(project),   # ← 用 ProcessRunner，不用 CommandRunner
        "release":    lambda: PublishRelease(runner, fs, events).run(project),
        "bench":      lambda: RunBenchmark(runner, fs, events).run(project),
        "coverage":   lambda: RunCoverage(runner, fs, events).run(project),
        # ...
    }

    handler = routes.get(args.command)
    if handler is None:
        parser.print_help()
        return 1

    ok = handler()
    return 0 if ok else 1
```

### 8.7 配置集中管理

丢一个 `config.json` 在 `kaubo-ops/` 下，纯数据，不写成任何特定工具的格式。Python stdlib `json` 零依赖加载。

```json
{
  "project": {
    "root": "."
  },
  "paths": {
    "rust_workspace": "next_kaubo",
    "gui_root": "next_kaubo/gui",
    "vscode_root": "vscode-extension"
  },
  "wasm": {
    "crate": "next_kaubo/crates/kaubo-wasm",
    "out_name": "kaubo_wasm",
    "targets": [
      {
        "target": "web",
        "output": "next_kaubo/gui/packages/wasm/pkg",
        "consumer": "GUI Playground"
      },
      {
        "target": "nodejs",
        "output": "vscode-extension/wasm",
        "consumer": "VSCode Extension"
      }
    ]
  },
  "release": {
    "version_file": ".version",
    "bump_default": "patch"
  },
  "tools": {
    "required": {
      "cargo": "1.70",
      "pnpm": "8",
      "npm": "9",
      "wasm-pack": "0.12"
    }
  }
}
```

领域层的 `KauboProject.__post_init__` 读取这份配置，代替硬编码路径。改目录结构时只改 json，不改 Python。

### 8.8 目录结构总览

```
packages/kaubo-features/
├── kaubo-ops/                     ← ★ Ops2 核心入口（`python kaubo-ops <cmd>`）
│   ├── __main__.py                # 入口：python kaubo-ops → 执行此文件
│   ├── config.json                # 集中配置
│   ├── cli/                       # 表示层
│   │   ├── __init__.py
│   │   └── main.py                # argparse + 命令路由
│   ├── app/                       # 应用层（用例）
│   │   ├── __init__.py
│   │   ├── ci_pipeline.py         # CiPipeline
│   │   ├── build_wasm.py          # BuildWasm
│   │   ├── build_all.py           # BuildAll
│   │   ├── dev_server.py          # DevServer
│   │   ├── publish_release.py
│   │   ├── deploy.py
│   │   ├── run_benchmark.py
│   │   └── run_coverage.py
│   ├── domain/                    # 领域层（模型）
│   │   ├── __init__.py
│   │   ├── project.py             # KauboProject 聚合根
│   │   ├── wasm.py                # WasmArtifact, WasmTarget
│   │   ├── rust.py                # RustWorkspace
│   │   ├── gui.py                 # GuiApp（裸命令，不经过 package.json scripts）
│   │   ├── vscode.py              # VscodeExtension（裸命令）
│   │   └── version.py             # ReleaseVersion
│   └── infra/                     # 基础设施层
│       ├── __init__.py
│       ├── command.py             # CommandRunner ABC + RealCommandRunner
│       ├── process.py             # ProcessRunner ABC + RealProcessRunner（长驻进程）
│       ├── filesystem.py          # FileSystem ABC + RealFileSystem
│       ├── events.py              # EventBus ABC + ConsoleEventBus
│       └── tools.py               # 工具版本检查
├── next_kaubo/                    # Rust workspace（纯源码，无 ops 入口）
│   ├── crates/
│   ├── gui/                       # Web Playground
│   │   └── packages/
│   │       ├── app/package.json   # ← 无 scripts 字段
│   │       ├── wasm/
│   │       └── types/package.json # ← 无 scripts 字段
│   ├── kaubo2-cli/
│   └── ops/                       # 旧 Python 脚本（Phase 2 后废弃）
├── vscode-extension/              # VSCode 扩展
│   ├── package.json               # ← 无 scripts 字段
│   └── ...
└── AGENTS.md                      # 指向 Ops2
```

### 8.9 与业界坐标系的对齐

把 Ops2 放在横坐标上，左边是"简单胶水"，右边是"重型工程化"：

| 方案 | 复杂度 | 编排语言 | 抽象层次 | 适合场景 |
|------|--------|---------|---------|---------|
| **Makefile / Justfile** | 极低 | Shell + 声明式依赖 | 任务清单 | 依赖简单、无分支逻辑 |
| **scripts/ 脚本集合** | 低 | Shell/Python 过程式 | 每个脚本管一摊 | 中等复杂度、团队小 |
| **xtask** (rust-analyzer, Helix) | 中 | Rust | 类型安全的函数调用 | 纯 Rust 项目，不需要跨语言编排 |
| **Ops2 (DDD Python)** | **中** | **Python + DDD 分层** | **领域模型 + 聚合根** | **跨语言、多目标、需长期维护的个人项目** |
| **Bazel / Pants** | 极高 | 自定义 DSL | 构建图 + 远程缓存 | 几十万行 monorepo，多语言，大团队 |

Ops2 在坐标系中的真实位置：

- **对标 Deno 的 `tools/` 目录**（用 Python 做构建脚本）——但 Deno 是过程式脚本集合，Ops2 有 DDD 分层。
- **对标 xtask**——但 Ops2 用 Python 代替 Rust 作为编排语言，因为 Kaubo 已经有 Python ops 积累（benchmark 的数据分析天然适合 Python），不需要跨语言调用。
- **对标大型 monorepo 的自定义构建 DSL**——但 Ops2 更轻，且复用 Python 生态。

**Ops2 在行业中属于"少数派但合理"的选择**。大多数项目要么 Make/Just（因为简单），要么 xtask（因为纯 Rust）。在"需要复杂逻辑但不想引入 Bazel"的夹缝地带，Python + DDD 是一种非常务实的工程化方案——它把"项目维护者的大脑"显式地用代码表达了出来。

### 8.10 为什么 DDD 在这个规模不嫌重

一个直接的问题：当前 ops/ 下所有 Python 脚本加起来 ~400 行，用四层 DDD 框架代码可能就超过 400 行，是否过度设计？

**不算过度，因为四个理由**：

1. **AI 写框架代码**：表示层的 argparse、基础设施层的 ABC、领域层的 dataclass——这些都是标准化模板。AI 生成它们只需几十秒，维护成本为零。

2. **业务逻辑不会停在 400 行**：Kaubo 在增长——新的 module system、VFS、LSP coordinator 都会带来新的构建/验证/部署步骤。DDD 的扩展方式（加一个领域对象 + 一个用例）比过程式脚本的扩展方式（在一堆 Shell 里 grep 路径）更可持续。

3. **框架行数是一次性投资**：`CommandRunner` ABC 写一次，所有未来的用例都受益于它的存在。而过程式脚本的"每个脚本自己 `subprocess.run`"是重复成本，随脚本数量线性增长。

4. **个人项目的特殊优势**：没有同事需要说服、没有 PR review 争论抽象层次。只要你自己六年后能看懂，它就是好架构。

### 8.11 迁移兼容性

Ops2 是新增层，不强行替换现有入口。阶段性共存，逐步收敛：

```
Phase 1 (当前):
    现状不变。cargo make ci、python3 ops/benchmark/... 仍然工作。
    子项目 package.json 中 scripts 字段仍然存在（但不再新增）。
    本阶段产出：本文档确认方案 + 业界对比 + DDD 架构蓝图。

Phase 2 (Ops2 核心落地):
    python kaubo-ops ci        ← 新入口可用，底层调用领域对象 + 基础设施
    python kaubo-ops bench     ← 应用层封装旧 ops/benchmark 逻辑
    python kaubo-ops dev       ← Ops2 托管 Vite 开发服务器（pnpm exec vite）
    旧入口仍然工作，Ops2 与旧脚本共存。

Phase 3 (子项目变哑):
    GUI/VSCode 的 package.json 中删除 scripts 字段。
    vscode-extension/build-wasm.sh 删除（功能由 BuildWasm 用例覆盖）。
    所有构建/测试/格式化走 Ops2。
    Makefile.toml 任务改为薄别名：["python", "kaubo-ops", "ci"]。

Phase 4 (清理旧路径，可选):
    删除 next_kaubo/ops/ 下的旧 Python 脚本。
    删除 cargo-make 依赖（或保留 Makefile.toml 做 CI 别名）。
    更新所有文档链接指向 Ops2。
```

每一阶段都不破坏前一阶段的行为。关键是 Phase 3——"子项目变哑"是一个不可逆的物理约束：一旦 `package.json` 中删除了 scripts 字段，就再也没有回头路。
