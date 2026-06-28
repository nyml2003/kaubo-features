"""Web Playground——纯静态资源，行为全部由 Ops2 定义。

对应的 package.json 无 scripts 字段（Phase 3 后生效）。
Ops2 直接执行裸命令（pnpm exec vite），不经过子项目包管理器脚本。

路径说明：
- root: app 包目录（next_kaubo/gui/packages/app）—— 大多数命令的 cwd
- workspace_root: pnpm workspace 根（next_kaubo/gui）—— e2e 等 workspace 级命令的 cwd
- types_dir: types 包目录（next_kaubo/gui/packages/types）—— 类型构建的 cwd
"""

from dataclasses import dataclass
from pathlib import Path


@dataclass
class GuiApp:
    """Web Playground——命令的领域对象。"""

    root: Path              # app 包目录（pnpm exec 从这里解析 binary）
    workspace_root: Path    # pnpm workspace 根（e2e 等全 workspace 命令）
    types_dir: Path         # types 包目录（tsc 编译）
    wasm_pkg: Path          # WASM 产物消费位置

    def dev_command(self) -> list[str]:
        """启动开发服务器（Ops2 托管进程，透传信号）。"""
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
        """构建共享类型包（@kaubo/types）——从 types 目录执行。"""
        return ["pnpm", "exec", "tsc", "-b"]
