"""性能分析 — 用 flamegraph 跑单个 benchmark"""
import os, sys, subprocess, tempfile
from pathlib import Path

ROOT = Path(__file__).parent.parent.parent
BENCH_DIR = Path(__file__).parent.parent / "bench"

def run_profile(suite, lang="kaubo", binary_mode=False):
    """Run a single suite with profiling."""
    if lang != "kaubo":
        print("Profile mode currently only supports kaubo.")
        return

    src = BENCH_DIR / "kaubo" / f"{suite}.kaubo"
    if not src.exists():
        print(f"File not found: {src}")
        return

    kaubo = _find_kaubo()
    print(f"Running profile for {suite}...")

    # Try flamegraph if available, otherwise just report time
    if _has_command("flamegraph"):
        _run_flamegraph(kaubo, src, suite)
    elif _has_command("perf"):
        _run_perf(kaubo, src, suite)
    else:
        print("  No profiling tools found (flamegraph/perf).")
        print("  Install: cargo install flamegraph")
        print(f"  Run manually: time {kaubo} {src}")

def _find_kaubo():
    for p in [ROOT / "target/release/kaubo-cli", ROOT / "target/debug/kaubo-cli"]:
        if p.exists(): return str(p)
    return "cargo"

def _has_command(cmd):
    return subprocess.run(["which", cmd], capture_output=True).returncode == 0

def _run_flamegraph(kaubo, src, suite):
    out = f"flamegraph_{suite}.svg"
    cmd = ["flamegraph", "-o", out, "--", kaubo, str(src)]
    subprocess.run(cmd, cwd=str(ROOT))
    print(f"  Flamegraph saved to {out}")

def _run_perf(kaubo, src, suite):
    out = f"perf_{suite}.data"
    subprocess.run(["perf", "record", "-o", out, "-g", "--", kaubo, str(src)], cwd=str(ROOT))
    print(f"  Perf data saved to {out}")
    print(f"  View: perf report -i {out}")
