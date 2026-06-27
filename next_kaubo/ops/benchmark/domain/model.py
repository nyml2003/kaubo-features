from dataclasses import dataclass, field
from pathlib import Path

@dataclass(frozen=True)
class Language:
    name: str
    ext: str
    cmd: str          # e.g. "python", "node", kaubo binary path

@dataclass(frozen=True)
class Case:
    """A single benchmark case — algorithm + expected output."""
    name: str
    path: Path         # directory containing main.{ext} files

    def expected_for(self, lang: str) -> str:
        """Read expected output, with optional per-language override.

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
    """One timed execution of a case×language."""
    elapsed_us: float

@dataclass
class BenchResult:
    case: Case
    language: Language
    runs: list[Run] = field(default_factory=list)

    @property
    def avg_us(self) -> float:
        return sum(r.elapsed_us for r in self.runs) / len(self.runs) if self.runs else 0

@dataclass
class Config:
    """Runner configuration."""
    suites_dir: Path
    iterations: int = 10
    warmup: int = 3
    timeout_s: int = 120
