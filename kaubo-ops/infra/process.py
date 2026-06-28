"""长驻进程管理抽象——用于 dev server 等需要托管生命周期的命令。

与 CommandRunner 互补：
- CommandRunner.run() = 一次性执行，等 exit code
- ProcessRunner.spawn() = 启动后立刻返回 ProcessHandle，托管生命周期
"""

import shutil
import subprocess
import sys
from abc import ABC, abstractmethod
from pathlib import Path


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

    @property
    @abstractmethod
    def pid(self) -> int:
        """操作系统 PID。"""
        ...


class ProcessRunner(ABC):
    """启动长驻进程的抽象——与 CommandRunner 互补。"""

    @abstractmethod
    def spawn(self, cmd: list[str], cwd: Path | None = None,
              env: dict[str, str] | None = None) -> ProcessHandle:
        ...


class RealProcessRunner(ProcessRunner):
    """真实的进程启动器——封装 subprocess.Popen。

    Windows 兼容：.CMD/.BAT 文件通过 cmd /c 执行。
    """

    def spawn(self, cmd: list[str], cwd: Path | None = None,
              env: dict[str, str] | None = None) -> ProcessHandle:
        resolved = self._resolve_windows(cmd)
        p = subprocess.Popen(resolved, cwd=cwd, env=env)
        return _RealProcessHandle(p)

    @staticmethod
    def _resolve_windows(cmd: list[str]) -> list[str]:
        """Windows: 如果命令是 .CMD/.BAT 文件，用 cmd /c 包装。"""
        if sys.platform != "win32" or not cmd:
            return cmd
        exe = shutil.which(cmd[0])
        if exe and exe.lower().endswith((".cmd", ".bat")):
            return ["cmd", "/c"] + cmd
        return cmd


class _RealProcessHandle(ProcessHandle):
    """subprocess.Popen 的适配器。"""

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
