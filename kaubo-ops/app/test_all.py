"""测试用例——Rust / Web / VSCode 及组合。"""

from domain.project import KauboProject
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class TestRust:
    """Rust 测试用例。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Rust test")
        r = self.runner.run(
            project.create_rust_workspace().test_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Rust test failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "Rust test passed")
        return True


class TestWeb:
    """Web 测试用例。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Web test")
        gui = project.create_gui_app()
        r = self.runner.run(gui.test_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"Web test failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "Web test passed")
        return True


class TestWebE2e:
    """Web e2e 测试用例。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Web e2e")
        gui = project.create_gui_app()
        r = self.runner.run(
            ["pnpm", "exec", "playwright", "test", "--config", "e2e/playwright.config.ts"],
            cwd=gui.workspace_root,
        )
        if not r.ok:
            self.events.emit("error", f"E2E test failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "E2E test passed")
        return True


class TestVscode:
    """VSCode 测试用例。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "VSCode test")
        vscode = project.create_vscode_extension()
        r = self.runner.run(vscode.test_command(), cwd=vscode.root)
        if not r.ok:
            self.events.emit("error", f"VSCode test failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "VSCode test passed")
        return True


class TestAll:
    """全部测试用例——Rust + Web + VSCode。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Test All")

        for test_case, name in [
            (TestRust(self.runner, self.fs, self.events), "Rust"),
            (TestWeb(self.runner, self.fs, self.events), "Web"),
            (TestVscode(self.runner, self.fs, self.events), "VSCode"),
        ]:
            if not test_case.run(project):
                self.events.emit("error", f"{name} test failed, stopping")
                return False

        self.events.emit("success", "All tests passed")
        return True
