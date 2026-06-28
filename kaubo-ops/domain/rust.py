"""Rust workspace——知道自己包含哪些 crate、怎么检查/测试/构建。"""

from dataclasses import dataclass
from pathlib import Path


@dataclass
class RustWorkspace:
    """Rust workspace——构建命令的领域对象。"""

    root: Path

    def check_command(self) -> list[str]:
        return ["cargo", "check", "--workspace", "--all-targets"]

    def test_command(self) -> list[str]:
        return ["cargo", "test", "--workspace"]

    def clippy_command(self) -> list[str]:
        return ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]

    def fmt_check_command(self) -> list[str]:
        return ["cargo", "fmt", "--all", "--", "--check"]

    def fmt_command(self) -> list[str]:
        return ["cargo", "fmt", "--all"]

    def build_release_command(self, package: str) -> list[str]:
        return ["cargo", "build", "--release", "-p", package]

    def doc_command(self) -> list[str]:
        return ["cargo", "doc", "--workspace", "--open"]
