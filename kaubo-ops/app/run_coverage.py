"""覆盖率用例——使用 cargo-llvm-cov 生成覆盖率报告。

从 next_kaubo/ops/quality/coverage.py 迁移逻辑。

依赖：
- rustup nightly
- cargo install cargo-llvm-cov
"""

import os
import subprocess
import sys
import webbrowser

from domain.project import KauboProject
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class RunCoverage:
    """覆盖率用例——cargo-llvm-cov + nightly，支持行和分支覆盖率。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject, html: bool = False, open_browser: bool = False) -> bool:
        self.events.emit("step", "Coverage Report")

        # 检查依赖
        if not self._check_cargo_llvm_cov():
            return False
        if not self._check_nightly():
            return False

        # 构建命令
        cmd = [
            "cargo", "+nightly", "llvm-cov",
            "--workspace",
            "--branch",
            "--all-features",
        ]

        output_dir = project.rust_workspace / "target" / "llvm-cov"
        html_file = output_dir / "index.html"

        if html or open_browser:
            cmd.extend(["--html", "--output-dir", str(output_dir)])

        self.events.emit("info", f"Command: {' '.join(cmd)}")

        r = self.runner.run(cmd, cwd=project.rust_workspace)
        if not r.ok:
            self.events.emit("error", f"Coverage failed:\n{r.stderr[-500:]}")
            return False

        if html or open_browser:
            self.events.emit("info", f"Report: {output_dir.resolve()}")
            self.events.emit("info", f"HTML: {html_file.resolve()}")

            if open_browser and html_file.exists():
                self.events.emit("info", "Opening browser...")
                webbrowser.open(f"file://{html_file.resolve()}")

        self.events.emit("success", "Coverage report generated")
        return True

    def _check_cargo_llvm_cov(self) -> bool:
        r = subprocess.run(["cargo", "llvm-cov", "--version"], capture_output=True, text=True)
        if r.returncode != 0:
            self.events.emit("error", "cargo-llvm-cov not installed")
            self.events.emit("info", "Install: cargo install cargo-llvm-cov")
            return False
        self.events.emit("info", f"cargo-llvm-cov: {r.stdout.strip()}")
        return True

    def _check_nightly(self) -> bool:
        r = subprocess.run(["rustup", "show"], capture_output=True, text=True)
        if "nightly" in r.stdout:
            self.events.emit("info", "nightly toolchain available")
            return True
        self.events.emit("error", "nightly toolchain not installed")
        self.events.emit("info", "Install: rustup install nightly")
        return False
