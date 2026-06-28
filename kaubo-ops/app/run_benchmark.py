"""Benchmark 用例——跨语言性能对比。

从 next_kaubo/ops/benchmark/ 迁移编排逻辑：
- 领域模型已在 domain/benchmark.py
- 本用例合并了旧 app/service.py 中的 bench 函数
- 基础设施发现已在旧 infra/discover.py，此处内联（简单逻辑）
"""

import os
import subprocess
import tempfile
import time
from pathlib import Path

from domain.project import KauboProject
from domain.benchmark import Language, Case, Config
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class RunBenchmark:
    """Benchmark 用例——对比 Kaubo 与 Python/Node 的性能。"""

    # 默认语言注册表（kaubo 的 cmd 在运行时注入）
    LANGUAGES = {
        "python": Language(name="python", ext="py", cmd="python"),
        "node":    Language(name="node",    ext="js", cmd="node"),
        "kaubo":   Language(name="kaubo",   ext="kaubo", cmd=""),
    }

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject, languages: list[str] | None = None,
            suites: list[str] | None = None, iters: int = 10,
            warmup: int = 3, kaubo_bin: str | None = None) -> bool:

        cfg = Config(
            suites_dir=project.benchmark_suites_dir,
            iterations=iters,
            warmup=warmup,
        )

        # 构建语言注册表
        langs = dict(self.LANGUAGES)
        if kaubo_bin:
            kb = Path(kaubo_bin)
            if not kb.exists():
                self.events.emit("error", f"Kaubo binary not found: {kb}")
                self.events.emit("info", "Build it: python kaubo-ops build-cli")
                return False
            langs["kaubo"] = Language(name="kaubo", ext="kaubo", cmd=str(kb.resolve()))
        else:
            # 尝试默认路径
            default_bin = project.rust_workspace / "target" / "release" / "kaubo2-cli"
            if default_bin.with_suffix(".exe").exists():
                default_bin = default_bin.with_suffix(".exe")
            if default_bin.exists():
                langs["kaubo"] = Language(name="kaubo", ext="kaubo", cmd=str(default_bin.resolve()))

        if languages:
            langs = {k: v for k, v in langs.items() if k in languages}

        if not langs:
            self.events.emit("error", f"No matching languages for {languages}")
            return False

        # 发现用例
        cases = self._discover_cases(cfg.suites_dir)
        if suites:
            cases = [c for c in cases if c.name in suites]
        if not cases:
            self.events.emit("error", f"No cases found in {cfg.suites_dir}")
            return False

        # 打印表头
        header = f"{'case':<12}"
        for name in langs:
            header += f" {name:>10}"
        print(header)

        errors: list[tuple[str, str, str]] = []

        for case in cases:
            print(f"{case.name:<12}", end="", flush=True)
            for lang_name in langs:
                try:
                    avg = self._bench_one(langs[lang_name], case, cfg)
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
            return False

        return True

    def _bench_one(self, lang: Language, case: Case, cfg: Config) -> float:
        if lang.name == "python":
            return self._bench_python(lang, case, cfg)
        elif lang.name == "node":
            return self._bench_node(lang, case, cfg)
        elif lang.name == "kaubo":
            return self._bench_kaubo(lang, case, cfg)
        else:
            raise ValueError(f"Unknown language: {lang.name}")

    # ── Discovery ───────────────────────────────────────────────

    def _discover_cases(self, suites_dir: Path) -> list[Case]:
        cases: list[Case] = []
        if not suites_dir.is_dir():
            return cases
        for d in sorted(suites_dir.iterdir()):
            if d.is_dir() and list(d.glob("main.*")):
                cases.append(Case(name=d.name, path=d))
        return cases

    # ── Python ──────────────────────────────────────────────────

    def _bench_python(self, lang: Language, case: Case, cfg: Config) -> float:
        self._validate_output(lang, case)
        src = case.path / "main.py"
        code = src.read_text(encoding="utf-8")
        lines = code.strip().split("\n")
        last = lines[-1].strip()
        body = "\n".join(lines[:-1])
        expr = last[len("print("):-1]
        wrapper = f'''
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
        r = subprocess.run(
            ["python", "-c", wrapper],
            capture_output=True, text=True, timeout=cfg.timeout_s,
        )
        if r.returncode != 0:
            raise RuntimeError(f"python/{case.name} failed: {r.stderr[:200]}")
        return float(r.stdout.strip().split()[-1])

    # ── Node ────────────────────────────────────────────────────

    def _bench_node(self, lang: Language, case: Case, cfg: Config) -> float:
        self._validate_output(lang, case)
        src = case.path / "main.js"
        code = src.read_text(encoding="utf-8")
        lines = code.strip().split("\n")
        last = lines[-1].strip()
        inner = last[len("console.log("):]
        if inner.endswith(")"):
            inner = inner[:-1]
        call_expr = inner.strip()

        wrapper = f'''
{code}
let _sink = [];
for (let i = 0; i < {cfg.warmup}; i++) _sink.push({call_expr});
_sink.length = 0;
let times = [];
for (let i = 0; i < {cfg.iterations}; i++) {{
    let t0 = performance.now();
    _sink.push({call_expr});
    times.push((performance.now() - t0) * 1000);
}}
if (_sink[0] === undefined) process.exit(1);
console.log(times.reduce((a,b) => a + b, 0) / times.length);
'''
        r = subprocess.run(
            ["node", "-e", wrapper],
            capture_output=True, text=True, timeout=cfg.timeout_s,
        )
        if r.returncode != 0:
            raise RuntimeError(f"node/{case.name} failed: {r.stderr[:200]}")
        return float(r.stdout.strip().split()[-1])

    # ── Kaubo ───────────────────────────────────────────────────

    def _bench_kaubo(self, lang: Language, case: Case, cfg: Config) -> float:
        if not lang.cmd or not Path(lang.cmd).exists():
            raise RuntimeError(f"kaubo binary not found: {lang.cmd}")
        self._validate_output(lang, case)
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

    # ── Validation ──────────────────────────────────────────────

    def _validate_output(self, lang: Language, case: Case) -> None:
        expected = case.expected_for(lang.name)
        if not expected:
            return

        src_path = case.path / f"main.{lang.ext}"

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
            if not lang.cmd or not Path(lang.cmd).exists():
                raise RuntimeError(f"kaubo binary not found: {lang.cmd}")
            r = subprocess.run(
                [lang.cmd, "run", str(src_path)],
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
