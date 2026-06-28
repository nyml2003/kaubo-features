"""开发服务器——长驻进程，透传 Ctrl-C 信号。"""

import signal

from domain.project import KauboProject
from infra.process import ProcessRunner
from infra.events import EventBus


class DevServer:
    """启动 Web 开发服务器——长驻进程，透传 Ctrl-C 信号。

    与 CI/Build 用例的关键区别：
    - CI/Build 用 CommandRunner.run()（同步等退出，适合一次性命令）
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
        def _forward_signal(signum, frame):
            self.events.emit("info", f"Received signal {signum}, forwarding to pid={handle.pid}")
            handle.terminate()

        original_sigint = signal.signal(signal.SIGINT, _forward_signal)
        original_sigterm = signal.signal(signal.SIGTERM, _forward_signal)
        try:
            return handle.wait()
        finally:
            signal.signal(signal.SIGINT, original_sigint)
            signal.signal(signal.SIGTERM, original_sigterm)
