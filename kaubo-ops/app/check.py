"""快速类型检查——Rust check + Web type build，不跑测试。"""

from domain.project import KauboProject
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class QuickCheck:
    """快速类型检查用例——不跑测试，只看编译是否通过。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject) -> bool:
        self.events.emit("step", "Quick Check (type-check only)")

        # Rust check
        self.events.emit("step", "Rust check")
        r = self.runner.run(
            project.create_rust_workspace().check_command(),
            cwd=project.rust_workspace,
        )
        if not r.ok:
            self.events.emit("error", f"Rust check failed:\n{r.stderr[-500:]}")
            return False

        # Web type build
        self.events.emit("step", "Web type build")
        gui = project.create_gui_app()
        self.runner.run(gui.types_build_command(), cwd=gui.types_dir)
        r = self.runner.run(
            ["pnpm", "exec", "tsc", "--noEmit"],
            cwd=gui.root,
        )
        if not r.ok:
            self.events.emit("error", f"Web type check failed:\n{r.stderr[-500:]}")
            return False

        self.events.emit("success", "Quick check passed")
        return True
