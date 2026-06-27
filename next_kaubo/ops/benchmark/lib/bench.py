"""Benchmark 引擎 — discovery + timing + validation"""
import os
import subprocess
import time
from pathlib import Path

# Force UTF-8 for subprocess I/O on all platforms
def _run(args, **kwargs):
    kwargs.setdefault("encoding", "utf-8")
    kwargs.setdefault("errors", "replace")
    return subprocess.run(args, **kwargs)

try:
    import tomllib
except ImportError:
    import toml as tomllib

from report import BenchResult

ROOT = Path(__file__).resolve().parents[3]
BENCH_DIR = Path(__file__).resolve().parents[1] / "suites"

# ---- Kaubo CLI resolution ----

KAUBO_CLI = None     # path to the built binary
KAUBO_BUILT = False  # whether we already triggered a build

def _ensure_kaubo_built(release=False):
    """Build kaubo binary once, return path. Cached globally."""
    global KAUBO_CLI, KAUBO_BUILT
    if KAUBO_CLI:
        return KAUBO_CLI

    profile = "release" if release else "debug"
    target = ROOT / "target" / profile / ("kaubo2-cli.exe" if os.name == "nt" else "kaubo2-cli")
    if not target.exists():
        print(f"  Building kaubo2 ({profile}) ...")
        r = _run(
            ["cargo", "build", "-p", "kaubo2-cli"] + (["--release"] if release else []),
            cwd=str(ROOT), capture_output=True, text=True, timeout=300        )
        if r.returncode != 0:
            raise RuntimeError(f"Cannot build kaubo2:\n{(r.stderr or '')[-500:]}")
        KAUBO_BUILT = True
    KAUBO_CLI = str(target)
    return KAUBO_CLI

# ---- Kaubo source → bytecode compilation (once) ----

def _compile_once(binary, src):
    """Compile .kaubo → .kaubod, return path to compiled binary."""
    out = str(src).replace(".kaubo", ".kaubod")
    # Only recompile if source changed
    if os.path.exists(out) and os.path.getmtime(out) >= os.path.getmtime(str(src)):
        return out, 0.0

    t0 = time.perf_counter()
    r = _run([binary, "compile", str(src)],
        capture_output=True, text=True, timeout=30, )
    compile_ms = (time.perf_counter() - t0) * 1000

    if r.returncode != 0:
        raise RuntimeError(f"Compilation failed for {src.name}:\n{(r.stderr or '')[:500]}")
    # Touch the output so mtime comparison works
    Path(out).touch()
    return out, compile_ms

# ---- Suite loading ----

def load_suites(config_path=None):
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
            if lang in ("description", "expected", "iterations", "warmup", "timeout"):
                continue
            suites[name]["languages"][lang] = dict(lang_cfg)
    return suites

def run_benchmarks(suites, languages=None, release=False):
    results = []
    for suite_name, suite in suites.items():
        if languages:
            suite_langs = {k: v for k, v in suite["languages"].items() if k in languages}
        else:
            suite_langs = suite["languages"]
        for lang, cfg in suite_langs.items():
            if lang == "kaubo":
                res = _run_kaubo(suite_name, cfg, suite["iterations"], suite["warmup"], suite["expected"], release)
            elif lang == "python":
                res = _run_python(suite_name, cfg, suite["iterations"], suite["warmup"], suite["expected"])
            elif lang == "rust":
                res = _run_rust(suite_name, cfg, suite["iterations"], suite["warmup"], suite["expected"])
            else:
                continue
            res.suite = suite_name
            res.lang = lang
            results.append(res)
    return results

# ---- Individual runners ----

def _run_kaubo(name, cfg, iterations, warmup, expected, release):
    binary = _ensure_kaubo_built(release)
    src = BENCH_DIR / "kaubo" / cfg.get("file", f"{name}.kaubo")

    times = []
    passed = True
    output = ""
    error = ""
    compile_ms = 0.0

    for i in range(warmup + iterations):
        t0 = time.perf_counter()
        r = _run([binary, str(src)],
            capture_output=True, text=True, timeout=30, )
        elapsed = (time.perf_counter() - t0) * 1000

        if r.returncode != 0:
            passed = False
            error = (r.stderr or '')[:500]
            break

        output = r.stdout.strip()
        # Strip trailing "= \<result\>" line added by kauba2-cli
        if '\n= ' in output:
            output = output[:output.rfind('\n= ')]
        if i == 0 and warmup > 0:
            compile_ms = elapsed  # first run includes .kaubo compilation
            continue
        times.append(elapsed)

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
    rust_dir = BENCH_DIR / "rust"
    manifest = rust_dir / "Cargo.toml"
    build = _run(
        ["cargo", "build", "--manifest-path", str(manifest), "--release"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=300    )
    if build.returncode != 0:
        return BenchResult(suite=name, lang="rust", times_ms=[], passed=False,
                          error=f"Rust build failed:\n{build.stderr[-300:]}")

    binary = rust_dir / "target" / "release" / "bench"
    suite_arg = cfg.get("function", name)

    # Rust binary handles internal timing — run once, parse avg_ns from last line
    internal_loops = cfg.get("loops", 1000)

    times = []
    passed = True
    output = ""
    error = ""

    for i in range(warmup + iterations):
        t0 = time.perf_counter()
        r = _run(
            [str(binary), suite_arg, str(internal_loops)],
            capture_output=True, text=True, timeout=300        )
        if r.returncode != 0:
            passed = False; error = (r.stderr or '')[:500]; break

        # Last line = avg_ns from internal timing
        lines = r.stdout.strip().split('\n')
        output = lines[-1]
        try:
            avg_ns = int(output)
            elapsed_ms = avg_ns / 1_000_000.0
        except ValueError:
            elapsed_ms = 0
            error = f"Cannot parse ns from: {output}"

        if i < warmup:
            continue
        times.append(elapsed_ms)

    return BenchResult(suite=name, lang="rust", times_ms=times, passed=passed, output=output, error=error)

# ---- Helpers ----

def _env():
    e = os.environ.copy()
    e["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + e.get("PATH", "")
    return e

def _validate_output(output, expected):
    try:
        if expected.startswith("float:"):
            return abs(float(output) - float(expected.split(":",1)[1])) < 1e-6
        if expected == "ok": return True  # any output is fine
        if expected == "'ok'":
            return output in ("ok", "'ok'") or "ok" in output
        return output == expected
    except (ValueError, TypeError):
        return output == expected
