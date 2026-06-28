"""表示层——CLI 入口和命令路由。

表示层只做三件事：解析参数、路由到用例、打印结果。
唯一的用户入口是 `python kaubo-ops <cmd>`。
"""

import argparse
import sys
from pathlib import Path

# 确保 kaubo-ops/ 在 sys.path 中，使 flat import 可用
_ops_root = Path(__file__).resolve().parents[1]
if str(_ops_root) not in sys.path:
    sys.path.insert(0, str(_ops_root))

from domain.project import KauboProject
from infra.command import RealCommandRunner
from infra.filesystem import RealFileSystem
from infra.events import ConsoleEventBus
from infra.process import RealProcessRunner


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Kaubo Ops2 — 领域驱动的工程编排系统",
    )
    sub = parser.add_subparsers(dest="command")

    # ── CI ─────────────────────────────────────────────────
    sub.add_parser("ci", help="标准 CI（check + lint + fmt + test + build）")
    sub.add_parser("ci-full", help="CI + e2e")

    # ── Check ──────────────────────────────────────────────
    sub.add_parser("check", help="快速类型检查（Rust + Web，无测试）")

    # ── Build ──────────────────────────────────────────────
    sub.add_parser("build", help="构建所有产物（WASM + CLI + Web + VSCode）")
    sub.add_parser("build-wasm", help="仅构建 WASM 双目标（web + nodejs）")
    sub.add_parser("build-cli", help="仅构建 CLI 二进制（release）")

    # ── Test ───────────────────────────────────────────────
    sub.add_parser("test", help="全部测试（Rust + Web + VSCode）")
    sub.add_parser("test-rust", help="Rust 测试")
    sub.add_parser("test-web", help="Web 单元测试")
    sub.add_parser("test-web-e2e", help="Web e2e 测试")
    sub.add_parser("test-vscode", help="VSCode 扩展测试")

    # ── Lint ───────────────────────────────────────────────
    sub.add_parser("lint", help="全部 lint（clippy + eslint）")
    sub.add_parser("lint-rust", help="Rust clippy")
    sub.add_parser("lint-web", help="Web eslint")

    # ── Fmt ────────────────────────────────────────────────
    sub.add_parser("fmt", help="全部格式化（rustfmt + prettier，写入模式）")
    sub.add_parser("fmt-check", help="全部格式检查（dry-run，不写入）")

    # ── Dev ────────────────────────────────────────────────
    sub.add_parser("dev", help="启动 Web 开发服务器（长驻进程，Ctrl-C 停止）")

    # ── Release ────────────────────────────────────────────
    rel = sub.add_parser("release", help="发布到 GitHub Release")
    rel.add_argument("version", nargs="?", help="直接指定版本号（不 auto bump）")
    rel.add_argument("--bump", choices=["major", "minor", "patch"], default="patch",
                     help="自动升版本（默认 patch）")
    rel.add_argument("-y", "--yes", action="store_true", help="跳过确认")

    # ── Deploy ─────────────────────────────────────────────
    dep = sub.add_parser("deploy", help="部署到 nginx")
    dep.add_argument("version", nargs="?", help="版本号（默认读 .version）")
    dep.add_argument("--root", type=Path, help="部署根目录")
    dep.add_argument("--nginx-conf", type=Path, help="nginx 配置目标路径")
    dep.add_argument("--repo", help="GitHub 仓库 (owner/repo)")

    # ── Benchmark ──────────────────────────────────────────
    bench = sub.add_parser("bench", help="运行跨语言性能对比")
    bench.add_argument("--lang", default="python,node",
                       help="逗号分隔的语言: python,node,kaubo (默认: python,node)")
    bench.add_argument("--suite", default="",
                       help="逗号分隔的用例名，如 fib,loop。默认全部")
    bench.add_argument("--iters", type=int, default=10, help="迭代次数")
    bench.add_argument("--warmup", type=int, default=3, help="预热次数")
    bench.add_argument("--kaubo-bin", default=None,
                       help="kaubo2-cli 二进制路径")

    # ── Coverage ───────────────────────────────────────────
    cov = sub.add_parser("coverage", help="生成覆盖率报告")
    cov.add_argument("--html", action="store_true", help="生成 HTML 报告")
    cov.add_argument("--open", action="store_true", help="生成并打开报告")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 1

    # ── 依赖注入 ──────────────────────────────────────────
    runner = RealCommandRunner()
    proc_runner = RealProcessRunner()
    fs = RealFileSystem()
    events = ConsoleEventBus()

    # KauboProject 根目录 = kaubo-ops 的父目录
    project_root = _ops_root.parent
    project = KauboProject(root=project_root)

    # ── 路由 ──────────────────────────────────────────────
    cmd = args.command

    if cmd == "ci":
        from app.ci_pipeline import CiPipeline
        ok = CiPipeline(runner, fs, events).run(project)
    elif cmd == "ci-full":
        from app.ci_pipeline import CiFullPipeline
        ok = CiFullPipeline(runner, fs, events).run(project)
    elif cmd == "check":
        from app.check import QuickCheck
        ok = QuickCheck(runner, fs, events).run(project)
    elif cmd == "build":
        from app.build import BuildAll
        ok = BuildAll(runner, fs, events).run(project)
    elif cmd == "build-wasm":
        from app.build import BuildWasm
        ok = BuildWasm(runner, fs, events).run(project)
    elif cmd == "build-cli":
        from app.build import BuildCli
        ok = BuildCli(runner, fs, events).run(project)
    elif cmd == "test":
        from app.test_all import TestAll
        ok = TestAll(runner, fs, events).run(project)
    elif cmd == "test-rust":
        from app.test_all import TestRust
        ok = TestRust(runner, fs, events).run(project)
    elif cmd == "test-web":
        from app.test_all import TestWeb
        ok = TestWeb(runner, fs, events).run(project)
    elif cmd == "test-web-e2e":
        from app.test_all import TestWebE2e
        ok = TestWebE2e(runner, fs, events).run(project)
    elif cmd == "test-vscode":
        from app.test_all import TestVscode
        ok = TestVscode(runner, fs, events).run(project)
    elif cmd == "lint":
        from app.lint_all import LintAll
        ok = LintAll(runner, fs, events).run(project)
    elif cmd == "lint-rust":
        from app.lint_all import LintRust
        ok = LintRust(runner, fs, events).run(project)
    elif cmd == "lint-web":
        from app.lint_all import LintWeb
        ok = LintWeb(runner, fs, events).run(project)
    elif cmd == "fmt":
        from app.fmt_all import FmtAll
        ok = FmtAll(runner, fs, events).run(project)
    elif cmd == "fmt-check":
        from app.fmt_all import FmtCheck
        ok = FmtCheck(runner, fs, events).run(project)
    elif cmd == "dev":
        from app.dev_server import DevServer
        exit_code = DevServer(proc_runner, events).run(project)
        return exit_code
    elif cmd == "release":
        from app.publish_release import PublishRelease
        ok = PublishRelease(runner, fs, events).run(
            project,
            bump=args.bump,
            version=args.version,
            skip_confirm=args.yes,
        )
    elif cmd == "deploy":
        from app.deploy import DeployApp
        ok = DeployApp(runner, fs, events).run(
            project,
            version=args.version,
            deploy_root=args.root,
            nginx_conf=args.nginx_conf,
            repo=args.repo,
        )
    elif cmd == "bench":
        from app.run_benchmark import RunBenchmark
        languages = [s.strip() for s in args.lang.split(",")]
        suites = [s.strip() for s in args.suite.split(",")] if args.suite else None
        ok = RunBenchmark(runner, fs, events).run(
            project,
            languages=languages,
            suites=suites,
            iters=args.iters,
            warmup=args.warmup,
            kaubo_bin=args.kaubo_bin,
        )
    elif cmd == "coverage":
        from app.run_coverage import RunCoverage
        ok = RunCoverage(runner, fs, events).run(
            project,
            html=args.html,
            open_browser=args.open,
        )
    else:
        parser.print_help()
        return 1

    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
