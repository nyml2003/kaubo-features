"""构建用例——WASM 双目标 + CLI + Web + VSCode。"""

from domain.project import KauboProject
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class BuildWasm:
    """构建 WASM 用例——一次构建，多目标产出。"""

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
                self.events.emit("error", f"WASM ({artifact.target.value}) failed:\n{r.stderr[-500:]}")
                return False
        self.events.emit("success", "WASM built for all targets")
        return True


class BuildCli:
    """构建 CLI 二进制用例。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Building CLI binary (release)")
        r = self.runner.run(
            project.create_rust_workspace().build_release_command("kaubo2-cli"),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"CLI build failed:\n{r.stderr[-500:]}")
            return False
        self.events.emit("success", "CLI built")
        return True


class BuildAll:
    """全量构建用例——WASM + CLI + Web + VSCode。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Build All")

        # WASM
        if not BuildWasm(self.runner, self.fs, self.events).run(project):
            return False

        # CLI
        if not BuildCli(self.runner, self.fs, self.events).run(project):
            return False

        # Web
        self.events.emit("step", "Web build")
        gui = project.create_gui_app()
        self.runner.run(gui.types_build_command(), cwd=gui.types_dir)
        r = self.runner.run(gui.build_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"Web build failed:\n{r.stderr[-500:]}")
            return False

        # VSCode (package)
        self.events.emit("step", "VSCode package")
        vscode = project.create_vscode_extension()
        r = self.runner.run(vscode.package_command(), cwd=vscode.root)
        if not r.ok:
            self.events.emit("error", f"VSCode package failed:\n{r.stderr[-500:]}")
            return False

        self.events.emit("success", "Build all passed")
        return True
