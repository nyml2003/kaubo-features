"""统一输出引擎 — table / JSON / markdown"""
import json, sys, time
from dataclasses import dataclass, field
from typing import Optional

COLORS = {"red": "\033[91m", "green": "\033[92m", "yellow": "\033[93m",
          "cyan": "\033[96m", "reset": "\033[0m", "bold": "\033[1m",
          "dim": "\033[2m"}

@dataclass
class BenchResult:
    suite: str
    lang: str
    times_ms: list  # raw run times
    compile_ms: float = 0.0  # compile time (kaubo only)
    passed: bool = True
    output: str = ""
    error: str = ""

    @property
    def median(self): return _median(self.times_ms) if self.times_ms else 0
    @property
    def mean(self): return sum(self.times_ms) / len(self.times_ms) if self.times_ms else 0
    @property
    def min(self): return min(self.times_ms) if self.times_ms else 0

@dataclass
class TestResult:
    name: str
    passed: bool
    output: str = ""
    error: str = ""
    exit_code: int = -1
    elapsed_ms: float = 0.0

@dataclass
class Summary:
    bench_results: list = field(default_factory=list)
    test_results: list = field(default_factory=list)
    @property
    def all_passed(self): return all(r.passed for r in self.bench_results + self.test_results)

def c(tag, text=""):
    if not sys.stdout.isatty(): return text
    return f"{COLORS.get(tag,'')}{text}{COLORS['reset']}"

def _median(data):
    s = sorted(data)
    n = len(s)
    if n == 0: return 0
    if n % 2: return s[n // 2]
    return (s[n // 2 - 1] + s[n // 2]) / 2

def _fmt_ms(ms):
    if ms < 1: return f"{ms*1000:.1f}us"
    if ms < 1000: return f"{ms:.1f}ms"
    return f"{ms/1000:.2f}s"

def print_bench_table(results: list[BenchResult]):
    """打印 Benchmark 对比表格"""
    suites = {}
    for r in results:
        suites.setdefault(r.suite, {})[r.lang] = r

    print(f"\n{c('bold')}=== Benchmark Results ==={c('reset')}\n")
    languages = sorted(set(r.lang for r in results))
    header = f"  {'Suite':<24}" + "".join(f"{l:<14}" for l in languages)
    print(c("bold") + header + c("reset"))

    for suite_name, lang_results in suites.items():
        row = f"  {suite_name:<24}"
        for lang in languages:
            r = lang_results.get(lang)
            if r and r.times_ms:
                row += f"{_fmt_ms(r.median):<14}"
            else:
                row += f"{'--':<14}"
        status = c("green", " ✓") if all(r.passed for r in lang_results.values()) else c("red", " ✗")
        print(row + status)

    print()
    _print_comparison(results)

def _print_comparison(results: list[BenchResult]):
    """Kaubo vs Python/Rust 对比"""
    kaubo = [r for r in results if r.lang == "kaubo(bin)"]
    python = [r for r in results if r.lang == "python"]
    rust = [r for r in results if r.lang == "rust"]

    if not kaubo: return

    kaubo_dict = {r.suite: r.median for r in kaubo}

    if python:
        ratios = [kaubo_dict[r.suite] / r.median for r in python if r.suite in kaubo_dict and r.median > 0]
        if ratios:
            gm = _geomean(ratios)
            label = c("yellow") if gm > 1 else c("green")
            print(f"  Kaubo vs Python (geomean): {label}{gm:.1f}x{'faster' if gm > 1 else 'slower'}{c('reset')}")

    if rust:
        ratios = [kaubo_dict[r.suite] / r.median for r in rust if r.suite in kaubo_dict and r.median > 0]
        if ratios:
            gm = _geomean(ratios)
            label = c("red") if gm > 1 else c("green")
            print(f"  Kaubo vs Rust   (geomean): {label}{gm:.1f}x{'slower' if gm > 1 else 'faster'}{c('reset')}")

def _geomean(values):
    from math import exp, log
    return exp(sum(log(v) for v in values) / len(values))

def print_test_table(results: list[TestResult]):
    """打印测试结果表格"""
    passed = sum(1 for r in results if r.passed)
    failed = len(results) - passed
    print(f"\n{c('bold')}=== Test Results{c('reset')}  {c('green', str(passed))} passed, {c('red' if failed else '', str(failed))} failed\n")
    for r in results:
        status = c("green", "PASS") if r.passed else c("red", "FAIL")
        print(f"  [{status}] {r.name}  ({_fmt_ms(r.elapsed_ms)})")
        if not r.passed and r.error:
            for line in r.error.strip().split('\n')[-3:]:
                print(f"         {c('dim', line)}")

def write_json(summary: Summary, path: str):
    """写入 JSON 报告"""
    data = {
        "benchmarks": [
            {"suite": r.suite, "lang": r.lang, "median_ms": r.median,
             "mean_ms": r.mean, "min_ms": r.min, "times_ms": r.times_ms,
             "compile_ms": r.compile_ms, "passed": r.passed}
            for r in summary.bench_results
        ],
        "tests": [
            {"name": r.name, "passed": r.passed, "elapsed_ms": r.elapsed_ms,
             "exit_code": r.exit_code}
            for r in summary.test_results
        ]
    }
    with open(path, 'w') as f:
        json.dump(data, f, indent=2)
    print(f"\n  Report written to {path}")
