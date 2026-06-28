"""Lint 用例——Rust clippy + Web eslint。"""

from domain.project import KauboProject
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class LintRust:
    """Rust Clippy lint 用例。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Rust clippy")
        r = self.runner.run(
            project.create_rust_workspace().clippy_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Clippy failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "Rust lint passed")
        return True


class LintWeb:
    """Web ESLint 用例。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Web eslint")
        gui = project.create_gui_app()
        r = self.runner.run(gui.lint_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"ESLint failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "Web lint passed")
        return True


class LintAll:
    """全部 lint 用例——Rust + Web。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Lint All")

        for lint_case, name in [
            (LintRust(self.runner, self.fs, self.events), "Rust"),
            (LintWeb(self.runner, self.fs, self.events), "Web"),
        ]:
            if not lint_case.run(project):
                self.events.emit("error", f"{name} lint failed, stopping")
                return False

        self.events.emit("success", "All lint passed")
        return True
