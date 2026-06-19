#!/usr/bin/env python3
"""Kaubo benchmark entry point.

Usage:
    python ops/benchmark/runner.py bench
    python ops/benchmark/runner.py bench --release
    python ops/benchmark/runner.py bench --lang kaubo python rust
    python ops/benchmark/runner.py bench --json
"""
import argparse
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "lib"))

parser = argparse.ArgumentParser(description="Kaubo unified test/benchmark tool")
sub = parser.add_subparsers(dest="command", required=True)

p_bench = sub.add_parser("bench", help="Run benchmarks")
p_bench.add_argument("--suite", help="Run specific suite")
p_bench.add_argument("--lang", nargs="+", help="Languages to run (kaubo python rust)")
p_bench.add_argument("--release", action="store_true", help="Use release build for kaubo/rust")
p_bench.add_argument("--json", action="store_true", help="Output JSON report")
p_bench.add_argument("--output", default="results/report.json", help="JSON output path")
p_bench.add_argument("--no-warmup", action="store_true")
p_bench.add_argument("--iterations", type=int, default=0)
p_bench.add_argument("--save-baseline", action="store_true",
    help="Save current results as performance baseline")
p_bench.add_argument("--check", action="store_true",
    help="Check against baseline for performance regressions")

args = parser.parse_args()

if args.command == "bench":
    from lib.bench import load_suites, run_benchmarks
    from lib.report import (print_bench_table, write_json, Summary,
                            save_baseline, print_baseline_check, check_baseline)

    suites = load_suites()
    if args.suite:
        suites = {args.suite: suites[args.suite]}
    if args.iterations > 0:
        for s in suites.values():
            s["iterations"] = args.iterations
    if args.no_warmup:
        for s in suites.values():
            s["warmup"] = 0

    results = run_benchmarks(suites, languages=args.lang, release=args.release)
    summary = Summary(bench_results=results)
    print_bench_table(results)

    if args.save_baseline:
        save_baseline(results)

    if args.check:
        violations = check_baseline(results)
        print_baseline_check(violations)
        if violations:
            sys.exit(1)

    if args.json:
        os.makedirs(os.path.dirname(args.output) or ".", exist_ok=True)
        write_json(summary, args.output)

    if not summary.all_passed:
        sys.exit(1)
