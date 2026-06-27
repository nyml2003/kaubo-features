"""Benchmark runner — entry point.

Usage:
  python runner.py --lang python,node --suite fact
  python runner.py --lang python,node
  python runner.py --suite fib,loop --iters 5
"""
import argparse
from pathlib import Path
from domain.model import Config
from app.service import bench_all, LANGUAGES

ROOT = Path(__file__).resolve().parent
SUITES = ROOT / "suites"
DEFAULT_KAUBO = ROOT.parent.parent / "target" / "release" / "kaubo2-cli.exe"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", default="python,node",
                    help="comma-separated: python,node,kaubo")
    ap.add_argument("--suite", default="",
                    help="comma-separated case names, e.g. fib,loop. Default: all")
    ap.add_argument("--iters", type=int, default=10)
    ap.add_argument("--warmup", type=int, default=3)
    ap.add_argument("--kaubo-bin", default=str(DEFAULT_KAUBO),
                    help="path to kaubo2-cli binary")
    args = ap.parse_args()

    languages = [s.strip() for s in args.lang.split(",")]
    suites = [s.strip() for s in args.suite.split(",")] if args.suite else None
    cfg = Config(suites_dir=SUITES, iterations=args.iters, warmup=args.warmup)

    if "kaubo" in languages:
        kb = Path(args.kaubo_bin)
        if not kb.exists():
            print(f"Kaubo binary not found: {kb}")
            print("Build it: cd .. && cargo build --release -p kaubo2-cli")
            return
        LANGUAGES["kaubo"] = LANGUAGES["kaubo"].__class__(
            name="kaubo", ext="kaubo", cmd=str(kb.resolve())
        )

    bench_all(cfg, languages, suites)


if __name__ == "__main__":
    main()
