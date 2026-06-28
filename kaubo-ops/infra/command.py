"""命令执行抽象——所有一次性外部命令的唯一入口。

领域层和应用层不直接调用 subprocess，通过此抽象执行。
"""

import shutil
import subprocess
import sys
from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path


@dataclass
class CommandResult:
    """一次命令执行的结果。"""
    exit_code: int
    stdout: str
    stderr: str

    @property
    def ok(self) -> bool:
        return self.exit_code == 0


class CommandRunner(ABC):
    """执行外部命令的抽象——唯一的外部副作用入口。"""

    @abstractmethod
    def run(self, cmd: list[str], cwd: Path | None = None,
            env: dict[str, str] | None = None) -> CommandResult:
        ...


class RealCommandRunner(CommandRunner):
    """真实的命令执行器——封装 subprocess.run。

    Windows 兼容：.CMD/.BAT 文件通过 cmd /c 执行，
    .EXE 文件直接调用 CreateProcess。
    """

    def run(self, cmd: list[str], cwd: Path | None = None,
            env: dict[str, str] | None = None) -> CommandResult:
        resolved = _resolve_windows_command(cmd)
        result = subprocess.run(
            resolved,
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        return CommandResult(
            exit_code=result.returncode,
            stdout=result.stdout,
            stderr=result.stderr,
        )


def _resolve_windows_command(cmd: list[str]) -> list[str]:
    """Windows: 如果命令是 .CMD/.BAT 文件，用 cmd /c 包装。

    其他平台直接原样返回。
    """
    if sys.platform != "win32":
        return cmd
    if not cmd:
        return cmd

    exe = shutil.which(cmd[0])
    if exe and exe.lower().endswith((".cmd", ".bat")):
        # cmd /c <original_cmd[0]> <arg1> <arg2> ...
        return ["cmd", "/c"] + cmd
    return cmd
