"""Benchmark 领域模型——从 next_kaubo/ops/benchmark/domain/model.py 迁移。

保持 frozen dataclass 设计，避免外部意外修改。
"""

from dataclasses import dataclass, field
from pathlib import Path


@dataclass(frozen=True)
class Language:
    """一种可被 benchmark 的编程语言。"""
    name: str
    ext: str
    cmd: str          # e.g. "python", "node", kaubo binary path


@dataclass(frozen=True)
class Case:
    """一个 benchmark 用例——算法 + 预期输出。"""
    name: str
    path: Path         # directory containing main.{ext} files

    def expected_for(self, lang: str) -> str:
        """读取预期输出，优先取特定语言，fallback 到通用。

        Checks `expected.<lang>.txt` first, then falls back to `expected.txt`.
        Returns "" if neither exists.
        """
        specific = self.path / f"expected.{lang}.txt"
        if specific.exists():
            return specific.read_text(encoding="utf-8").strip()
        general = self.path / "expected.txt"
        if general.exists():
            return general.read_text(encoding="utf-8").strip()
        return ""


@dataclass
class Run:
    """一次计时的执行记录。"""
    elapsed_us: float


@dataclass
class BenchResult:
    """一个 case × language 的 benchmark 结果。"""
    case: Case
    language: Language
    runs: list[Run] = field(default_factory=list)

    @property
    def avg_us(self) -> float:
        return sum(r.elapsed_us for r in self.runs) / len(self.runs) if self.runs else 0


@dataclass
class Config:
    """Benchmark runner 配置。"""
    suites_dir: Path
    iterations: int = 10
    warmup: int = 3
    timeout_s: int = 120
