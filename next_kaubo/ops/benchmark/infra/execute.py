import subprocess, time
from domain.model import Language, Case, Run


def run_once(lang: Language, case: Case) -> str:
    """Execute one case in one language. Returns stdout on success, raises on failure."""
    src = case.path / f"main.{lang.ext}"
    r = subprocess.run(
        [lang.cmd, str(src)],
        capture_output=True, text=True, timeout=30
    )
    if r.returncode != 0:
        raise RuntimeError(f"{case.name}/{lang.name} failed: {r.stderr[:200]}")
    return r.stdout.strip()


def time_run(lang: Language, case: Case) -> float:
    """Time a single execution, return elapsed microseconds."""
    src = case.path / f"main.{lang.ext}"
    t0 = time.perf_counter()
    subprocess.run(
        [lang.cmd, str(src)],
        capture_output=True, text=True, timeout=30,
        check=True
    )
    return (time.perf_counter() - t0) * 1_000_000
