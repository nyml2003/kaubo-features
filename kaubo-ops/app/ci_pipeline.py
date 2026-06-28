"""标准 CI 管线——check + lint + fmt + test + build，全目标。

CI 流程顺序一目了然——它就是 Kaubo CI 的"可执行文档"。
"""

from domain.project import KauboProject
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus
from infra.tools import check_tools


class CiPipeline:
    """标准 CI 用例：check + test + lint，全目标。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "CI Pipeline")

        # 1. 环境检查
        missing = check_tools(["cargo", "pnpm", "wasm-pack"])
        if missing:
            self.events.emit("error", f"Missing tools: {', '.join(missing)}")
            return False

        # 2. Rust check
        self.events.emit("step", "Rust check")
        r = self.runner.run(
            project.create_rust_workspace().check_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Rust check failed:\n{r.stderr[-500:]}")
            return False

        # 3. Rust clippy
        self.events.emit("step", "Rust clippy")
        r = self.runner.run(
            project.create_rust_workspace().clippy_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Clippy failed:\n{r.stderr[-500:]}")
            return False

        # 4. Rust fmt check
        self.events.emit("step", "Rust fmt check")
        r = self.runner.run(
            project.create_rust_workspace().fmt_check_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", "Formatting check failed — run 'python kaubo-ops fmt'")
            return False

        # 5. Rust test
        self.events.emit("step", "Rust test")
        r = self.runner.run(
            project.create_rust_workspace().test_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Rust test failed:\n{r.stderr[-500:]}")
            return False

        # 6. WASM build
        from app.build import BuildWasm
        if not BuildWasm(self.runner, self.fs, self.events).run(project):
            return False

        # 7. Web test
        self.events.emit("step", "Web test")
        gui = project.create_gui_app()
        r = self.runner.run(gui.test_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"Web test failed:\n{r.stderr[-500:]}")
            return False

        # 8. Web build
        self.events.emit("step", "Web build")
        self.runner.run(gui.types_build_command(), cwd=gui.types_dir)
        r = self.runner.run(gui.build_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"Web build failed:\n{r.stderr[-500:]}")
            return False

        # 9. VSCode test
        self.events.emit("step", "VSCode test")
        vscode = project.create_vscode_extension()
        r = self.runner.run(vscode.test_command(), cwd=vscode.root)
        if not r.ok:
            self.events.emit("error", f"VSCode test failed:\n{r.stderr[-500:]}")
            return False

        self.events.emit("success", "CI passed")
        return True


class CiFullPipeline:
    """CI + e2e 用例。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        # 先跑标准 CI
        if not CiPipeline(self.runner, self.fs, self.events).run(project):
            return False

        # e2e
        self.events.emit("step", "Web e2e")
        gui = project.create_gui_app()
        r = self.runner.run(
            ["pnpm", "exec", "playwright", "test", "--config", "e2e/playwright.config.ts"],
            cwd=gui.workspace_root,
        )
        if not r.ok:
            self.events.emit("error", f"E2E test failed:\n{r.stderr[-500:]}")
            return False

        self.events.emit("success", "CI-full passed")
        return True
