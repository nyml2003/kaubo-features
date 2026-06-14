#!/usr/bin/env python3
"""Kaubo 统一测试/基准工具入口

Usage:
    python tools/runner.py bench                           # 跑全部 benchmark (debug)
    python tools/runner.py bench --release                 # release 模式
    python tools/runner.py bench --lang kaubo python rust  # 三路对比
    python tools/runner.py bench --json                    # 输出 JSON
    python tools/runner.py test                            # 集成测试
    python tools/runner.py profile --suite mandelbrot      # 性能分析
"""
import sys, os, argparse
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

p_test = sub.add_parser("test", help="Run integration tests")

p_profile = sub.add_parser("profile", help="Run profiling")
p_profile.add_argument("--suite", required=True)

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

elif args.command == "test":
    from lib.test import load_tests, run_tests
    from lib.report import print_test_table, Summary

    tests, suite_config = load_tests()
    results = run_tests(tests, suite_config)
    summary = Summary(test_results=results)
    print_test_table(results)

    if not summary.all_passed:
        sys.exit(1)

elif args.command == "profile":
    from lib.profile import run_profile
    run_profile(args.suite)
