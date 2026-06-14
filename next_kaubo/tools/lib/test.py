"""集成测试引擎 — run examples, check exit code/output"""
import os, sys, time, subprocess
from pathlib import Path
try:
    import tomllib
except ImportError:
    import toml as tomllib
from report import TestResult

ROOT = Path(__file__).parent.parent.parent
KAUBO_CLI = None

def _find_kaubo():
    global KAUBO_CLI
    if KAUBO_CLI: return KAUBO_CLI
    for p in [ROOT / "target/release/kaubo", ROOT / "target/debug/kaubo"]:
        if p.exists():
            KAUBO_CLI = str(p); return KAUBO_CLI
    for p in [os.path.expanduser("~/.cargo/bin/cargo"), "cargo"]:
        if os.path.exists(p):
            KAUBO_CLI = p; return p
    return "cargo"

def load_tests(config_path=None):
    """加载集成测试配置"""
    if config_path is None:
        config_path = Path(__file__).parent.parent / "test" / "examples.toml"
    with open(config_path, 'rb') as f:
        data = tomllib.load(f)
    return (data.get("tests", []), data.get("suite", {}))

def run_tests(tests, suite_config):
    """运行所有集成测试, 返回 TestResult 列表"""
    _build_if_needed()
    results = []
    timeout = suite_config.get("timeout", 30)

    for test in tests:
        name = test.get("name", "unnamed")
        entry = test.get("entry", "")
        expected_output = test.get("expected_output", None)
        check_error = test.get("check_error", False)

        t0 = time.perf_counter()
        passed, output, error, exit_code = _run_one(entry, timeout, check_error)
        elapsed = (time.perf_counter() - t0) * 1000

        if expected_output is not None and expected_output not in output:
            passed = False
            error = f"Expected output '{expected_output}' not found in:\n{output}"

        results.append(TestResult(name=name, passed=passed, output=output,
                                  error=error, exit_code=exit_code, elapsed_ms=elapsed))
    return results

def _build_if_needed():
    kaubo = _find_kaubo()
    if kaubo == "cargo":
        r = subprocess.run(["cargo", "build", "-p", "kaubo-cli"],
            capture_output=True, text=True, timeout=300, cwd=str(ROOT))
        if r.returncode != 0:
            raise RuntimeError(f"Cargo build failed:\n{r.stderr[-500:]}")

def _run_one(entry, timeout, check_error):
    kaubo = _find_kaubo()
    if _is_cargo(kaubo):
        cmd = [kaubo, "run", "-p", "kaubo-cli", "--", str(ROOT / entry)]
    else:
        cmd = [kaubo, str(ROOT / entry)]

    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, cwd=str(ROOT), env=_env())
    except subprocess.TimeoutExpired:
        return False, "", f"Timeout after {timeout}s", -1
    except FileNotFoundError:
        return False, "", "Kaubo binary not found", -2

    output = r.stdout.strip()
    error = r.stderr.strip()

    if check_error:
        passed = r.returncode != 0 and error != ""
    else:
        passed = r.returncode == 0

    return passed, output, error, r.returncode

def _is_cargo(path):
    return path.endswith("cargo")

def _env():
    e = os.environ.copy()
    e["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + e.get("PATH", "")
    return e
