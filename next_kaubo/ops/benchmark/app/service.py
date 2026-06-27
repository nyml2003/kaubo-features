import subprocess, tempfile, os
from pathlib import Path
from domain.model import Language, Case, Config
from infra.discover import discover_cases


LANGUAGES = {
    "python": Language(name="python", ext="py", cmd="python"),
    "node":    Language(name="node",    ext="js", cmd="node"),
    "kaubo":   Language(name="kaubo",   ext="kaubo", cmd=""),
}


# ── Output validation ──────────────────────────────────────────

def _validate_output(lang: Language, case: Case) -> None:
    """Run the case once and compare its output to expected.txt.

    Checks `expected.<lang>.txt` first, falls back to `expected.txt`.
    For Kaubo, the ``= <result>`` trailer line is stripped before comparison.
    Raises RuntimeError if the output differs or the run fails.
    Skipped silently when no expected file exists.
    """
    expected = case.expected_for(lang.name)
    if not expected:
        return

    if lang.name == "python":
        r = subprocess.run(
            ["python", str(case.path / "main.py")],
            capture_output=True, text=True, timeout=30,
        )
    elif lang.name == "node":
        r = subprocess.run(
            ["node", str(case.path / "main.js")],
            capture_output=True, text=True, timeout=30,
        )
    elif lang.name == "kaubo":
        src = str((case.path / "main.kaubo").resolve())
        if not lang.cmd or not Path(lang.cmd).exists():
            raise RuntimeError(f"kaubo binary not found: {lang.cmd}")
        r = subprocess.run(
            [lang.cmd, "run", src],
            capture_output=True, text=True, timeout=30,
        )
    else:
        return

    if r.returncode != 0:
        raise RuntimeError(
            f"validate {case.name}/{lang.name} failed (rc={r.returncode}): "
            f"{r.stderr[:200]}"
        )

    actual = r.stdout.strip()
    if actual != expected:
        raise RuntimeError(
            f"output mismatch for {case.name}/{lang.name}:\n"
            f"  expected: {expected}\n"
            f"  got:      {actual}"
        )


# ── Python ─────────────────────────────────────────────────────

def _wrap_script(case: Case, cfg: Config) -> str:
    """Generate a timing wrapper that runs the case N times and prints avg_us."""
    src = case.path / "main.py"
    code = src.read_text(encoding="utf-8")
    lines = code.strip().split("\n")
    last = lines[-1].strip()
    body = "\n".join(lines[:-1])
    # Extract the expression inside print(...)
    expr = last[len("print("):-1]  # e.g. "fib(40)"
    return f'''
import time
{body}
def _bench():
    return {expr}
for _ in range({cfg.warmup}):
    _bench()
times = []
for _ in range({cfg.iterations}):
    t0 = time.perf_counter()
    _bench()
    times.append((time.perf_counter() - t0) * 1e6)
print(sum(times) / len(times))
'''


def bench_python(case: Case, cfg: Config) -> float:
    """Generate wrapper, run in single Python process."""
    _validate_output(LANGUAGES["python"], case)
    script = _wrap_script(case, cfg)
    r = subprocess.run(
        ["python", "-c", script],
        capture_output=True, text=True, timeout=cfg.timeout_s,
    )
    if r.returncode != 0:
        raise RuntimeError(f"python/{case.name} failed: {r.stderr[:200]}")
    return float(r.stdout.strip().split()[-1])


# ── Node ───────────────────────────────────────────────────────

def bench_node(case: Case, cfg: Config) -> float:
    """Run Node with inline timing loop — one process, N iterations."""
    _validate_output(LANGUAGES["node"], case)
    src = case.path / "main.js"
    code = src.read_text(encoding="utf-8")
    # Wrap: call function N times, time each, print avg in μs
    wrapper = f'''
{code}
let _fn = {case.name};  // by convention, the exported function has the case name
for (let i = 0; i < {cfg.warmup}; i++) _fn();
let times = [];
for (let i = 0; i < {cfg.iterations}; i++) {{
    let t0 = performance.now();
    _fn();
    times.push((performance.now() - t0) * 1000);
}}
console.log(times.reduce((a,b) => a + b, 0) / times.length);
'''
    r = subprocess.run(
        ["node", "-e", wrapper],
        capture_output=True, text=True, timeout=cfg.timeout_s,
    )
    if r.returncode != 0:
        raise RuntimeError(f"node/{case.name} failed: {r.stderr[:200]}")
    return float(r.stdout.strip().split()[-1])


# ── Kaubo ──────────────────────────────────────────────────────

def bench_kaubo(case: Case, cfg: Config) -> float:
    """Kaubo: bench subcommand does internal timing."""
    lang = LANGUAGES["kaubo"]
    if not lang.cmd or not Path(lang.cmd).exists():
        raise RuntimeError(f"kaubo binary not found: {lang.cmd}")
    _validate_output(lang, case)
    src = (case.path / "main.kaubo").resolve()
    out = tempfile.NamedTemporaryFile(delete=False, suffix=".txt", prefix=f"bench_{case.name}_")
    out.close()
    with open(out.name, "w") as f:
        ret = subprocess.run(
            [lang.cmd, "bench", str(src), str(cfg.iterations), str(cfg.warmup)],
            stdout=f, stderr=subprocess.STDOUT,
            timeout=cfg.timeout_s,
        ).returncode
    output = Path(out.name).read_text(encoding="utf-8", errors="replace")
    os.unlink(out.name)
    if ret != 0:
        raise RuntimeError(f"kaubo/{case.name} failed: {output[:200]}")
    for line in reversed(output.strip().split("\n")):
        parts = line.strip().split()
        try:
            return float(parts[0])
        except ValueError:
            continue
    raise RuntimeError(f"kaubo/{case.name}: no avg in output: {output[:200]}")


# ── Orchestration ──────────────────────────────────────────────

def bench_all(cfg: Config, languages: list[str], suites: list[str] | None = None):
    langs = {k: v for k, v in LANGUAGES.items() if k in languages}
    if not langs:
        print(f"No matching languages for {languages}")
        return

    cases = discover_cases(cfg.suites_dir)
    if suites:
        cases = [c for c in cases if c.name in suites]
    if not cases:
        print(f"No cases found in {cfg.suites_dir}")
        return

    print(f"{'case':<12}", end="")
    for name in langs:
        print(f" {name:>10}", end="")
    print()

    errors: list[tuple[str, str, str]] = []

    for case in cases:
        print(f"{case.name:<12}", end="", flush=True)
        for lang_name in langs:
            try:
                if lang_name == "python":
                    avg = bench_python(case, cfg)
                elif lang_name == "node":
                    avg = bench_node(case, cfg)
                elif lang_name == "kaubo":
                    avg = bench_kaubo(case, cfg)
                else:
                    avg = 0
                print(f" {avg:>9.1f}us", end="", flush=True)
            except Exception as e:
                print(f" {'ERR':>10}", end="", flush=True)
                errors.append((case.name, lang_name, str(e)))
        print()

    if errors:
        print(f"\n{'─' * 50}")
        print(f"{len(errors)} error(s):")
        for case_name, lang_name, msg in errors:
            print(f"  [{case_name}/{lang_name}] {msg}")
