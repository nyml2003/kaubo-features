"""Benchmark 引擎 — discovery + timing + validation"""
import os, sys, time, subprocess, statistics
from pathlib import Path
try:
    import tomllib
except ImportError:
    import toml as tomllib
from report import BenchResult

ROOT = Path(__file__).parent.parent.parent
BENCH_DIR = Path(__file__).parent.parent / "bench"
KAUBO_CLI = None  # resolved lazily

def _find_kaubo():
    global KAUBO_CLI
    if KAUBO_CLI:
        return KAUBO_CLI
    for p in [os.path.expanduser("~/.cargo/bin/cargo"), ROOT / "target/release/kaubo-cli", ROOT / "target/debug/kaubo-cli"]:
        if os.path.exists(p):
            KAUBO_CLI = str(p); return KAUBO_CLI
    KAUBO_CLI = "cargo"; return KAUBO_CLI

def load_suites(config_path=None):
    """加载 benchmark 配置"""
    if config_path is None:
        config_path = BENCH_DIR / "suites.toml"
    with open(config_path, 'rb') as f:
        data = tomllib.load(f)

    suites = {}
    for name, cfg in data.items():
        suites[name] = {
            "description": cfg.get("description", ""),
            "expected": str(cfg.get("expected", "")),
            "iterations": cfg.get("iterations", 5),
            "warmup": cfg.get("warmup", 1),
            "languages": {}
        }
        for lang, lang_cfg in cfg.items():
            if lang in ("description", "expected", "iterations", "warmup"):
                continue
            suites[name]["languages"][lang] = dict(lang_cfg)
    return suites

def run_benchmarks(suites, languages=None, binary_mode=True):
    """运行全部或指定的 benchmarks, 返回 BenchResult 列表"""
    results = []

    for suite_name, suite in suites.items():
        if languages:
            suite_langs = {k: v for k, v in suite["languages"].items() if k in languages}
        else:
            suite_langs = suite["languages"]

        for lang, cfg in suite_langs.items():
            if lang == "kaubo":
                res = _run_kaubo(suite_name, cfg, suite["iterations"], suite["warmup"], suite["expected"], binary_mode)
            elif lang == "python":
                res = _run_python(suite_name, cfg, suite["iterations"], suite["warmup"], suite["expected"])
            elif lang == "rust":
                res = _run_rust(suite_name, cfg, suite["iterations"], suite["warmup"], suite["expected"])
            else:
                continue

            res.suite = suite_name
            res.lang = lang + ("(bin)" if lang == "kaubo" and binary_mode else "")
            results.append(res)

    return results

def _run_kaubo(name, cfg, iterations, warmup, expected, binary_mode):
    kaubo = _find_kaubo()
    src = BENCH_DIR / "kaubo" / cfg.get("file", f"{name}.kaubo")

    compile_ms = 0.0
    times = []
    passed = True
    output = ""
    error = ""

    for i in range(warmup + iterations):
        t0 = time.perf_counter()
        if _is_cargo(kaubo):
            r = subprocess.run([kaubo, "run", "-p", "kaubo-cli", "--", str(src)],
                capture_output=True, text=True, timeout=120, cwd=str(ROOT), env=_env())
        else:
            r = subprocess.run([kaubo, str(src)],
                capture_output=True, text=True, timeout=120, cwd=str(ROOT), env=_env())
        elapsed = (time.perf_counter() - t0) * 1000

        if r.returncode != 0:
            passed = False
            error = r.stderr[:500]
            break

        output = r.stdout.strip()
        if i == 0 and warmup > 0:
            compile_ms = elapsed  # first run includes compile
            continue

        times.append(elapsed)

    # Validate output
    if passed and expected:
        passed = _validate_output(output, expected)

    return BenchResult(suite=name, lang="kaubo", times_ms=times,
                       compile_ms=compile_ms, passed=passed, output=output, error=error)

def _run_python(name, cfg, iterations, warmup, expected):
    import importlib.util
    spec = importlib.util.spec_from_file_location("bench", BENCH_DIR / "python" / "bench.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)

    func = getattr(mod, cfg.get("function", name))
    args = cfg.get("args", [])

    times = []
    passed = True
    output = ""
    error = ""

    for i in range(warmup + iterations):
        t0 = time.perf_counter()
        try:
            result = func(*args)
        except Exception as e:
            passed = False; error = str(e); break
        elapsed = (time.perf_counter() - t0) * 1000
        if i < warmup: continue
        times.append(elapsed)
        output = str(result)

    if passed and expected:
        passed = _validate_output(output, expected)

    return BenchResult(suite=name, lang="python", times_ms=times, passed=passed, output=output, error=error)

def _run_rust(name, cfg, iterations, warmup, expected):
    # compile first
    rust_dir = BENCH_DIR / "rust"
    build = subprocess.run(["cargo", "build", "--release"], cwd=str(rust_dir),
                           capture_output=True, text=True, timeout=300)
    if build.returncode != 0:
        return BenchResult(suite=name, lang="rust", times_ms=[], passed=False,
                          error=f"Rust build failed:\n{build.stderr[-300:]}")

    binary = rust_dir / "target" / "release" / "bench"
    args = cfg.get("args", [])

    times = []
    passed = True
    output = ""
    error = ""

    for i in range(warmup + iterations):
        t0 = time.perf_counter()
        r = subprocess.run([str(binary)] + [str(a) for a in args],
            capture_output=True, text=True, timeout=300)
        elapsed = (time.perf_counter() - t0) * 1000
        if r.returncode != 0:
            passed = False; error = r.stderr[:500]; break
        if i < warmup: continue
        times.append(elapsed)
        output = r.stdout.strip().split('\n')[-1]  # last line = result

    if passed and expected:
        passed = _validate_output(output, expected)

    return BenchResult(suite=name, lang="rust", times_ms=times, passed=passed, output=output, error=error)

def _is_cargo(path):
    return path.endswith("cargo")

def _env():
    e = os.environ.copy()
    e["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + e.get("PATH", "")
    return e

def _validate_output(output, expected):
    try:
        if expected.startswith("float:"):
            return abs(float(output) - float(expected.split(":",1)[1])) < 1e-6
        if expected == "ok": return output == "ok"
        return output == expected
    except (ValueError, TypeError):
        return output == expected
