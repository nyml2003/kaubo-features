"""格式化用例——Rust rustfmt + Web prettier。"""

from domain.project import KauboProject
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class FmtRust:
    """Rust rustfmt 格式化（写入模式）。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Rust fmt")
        r = self.runner.run(
            project.create_rust_workspace().fmt_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Rust fmt failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "Rust formatted")
        return True


class FmtWeb:
    """Web prettier 格式化（写入模式）。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Web prettier")
        gui = project.create_gui_app()
        r = self.runner.run(gui.fmt_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"Prettier failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "Web formatted")
        return True


class FmtAll:
    """全部格式化用例——Rust + Web（写入模式）。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Fmt All")

        for fmt_case, name in [
            (FmtRust(self.runner, self.fs, self.events), "Rust"),
            (FmtWeb(self.runner, self.fs, self.events), "Web"),
        ]:
            if not fmt_case.run(project):
                self.events.emit("error", f"{name} fmt failed, stopping")
                return False

        self.events.emit("success", "All formatted")
        return True


class FmtCheck:
    """格式检查用例——Rust + Web（dry-run 模式，不写入）。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Fmt Check (dry-run)")

        # Rust fmt check
        self.events.emit("info", "Checking Rust formatting...")
        r = self.runner.run(
            project.create_rust_workspace().fmt_check_command(),
            cwd=project.rust_workspace,
        )
        rust_ok = r.ok

        # Web prettier check
        self.events.emit("info", "Checking Web formatting...")
        gui = project.create_gui_app()
        r2 = self.runner.run(gui.fmt_check_command(), cwd=gui.root)
        web_ok = r2.ok

        if not rust_ok:
            self.events.emit("error", "Rust formatting issues found — run 'python kaubo-ops fmt'")
        if not web_ok:
            self.events.emit("error", "Web formatting issues found — run 'python kaubo-ops fmt'")

        if rust_ok and web_ok:
            self.events.emit("success", "All formatting checks passed")
            return True
        return False
