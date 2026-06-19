#! /usr/bin/env python3
"""Kaubo 发布脚本 —— 自动升版本号、构建前端并发布到 GitHub Release。

用法:
    python3 ops/release/publish.py
    python3 ops/release/publish.py -y
    python3 ops/release/publish.py --bump minor
    python3 ops/release/publish.py --bump major
    python3 ops/release/publish.py 0.5.0

前提: 安装了 pnpm / gh CLI (gh auth login)
"""

import argparse
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
GUI_DIR = REPO_ROOT / "gui" / "packages" / "app"
DIST_DIR = GUI_DIR / "dist"


def read_version() -> str:
    version_file = REPO_ROOT / ".version"
    if not version_file.exists():
        sys.exit("错误: 找不到 .version 文件")
    version = version_file.read_text().strip()
    if not version:
        sys.exit("错误: .version 为空")
    return version


def write_version(version: str) -> None:
    (REPO_ROOT / ".version").write_text(version + "\n")


def bump_version(current: str, level: str) -> str:
    parts = current.split(".")
    if len(parts) != 3:
        sys.exit(f"错误: 版本号格式不对 — {current} (需要 X.Y.Z)")
    major, minor, patch = int(parts[0]), int(parts[1]), int(parts[2])
    if level == "major":
        return f"{major + 1}.0.0"
    elif level == "minor":
        return f"{major}.{minor + 1}.0"
    else:
        return f"{major}.{minor}.{patch + 1}"


def run(cmd: list[str], cwd: Path | None = None) -> None:
    print(f"  → {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd or REPO_ROOT)
    if result.returncode != 0:
        sys.exit(f"错误: 命令失败 (exit {result.returncode}): {' '.join(cmd)}")


def check_prerequisites() -> None:
    if not shutil.which("pnpm"):
        sys.exit("错误: 需要安装 pnpm (https://pnpm.io)")
    if not shutil.which("gh"):
        sys.exit("错误: 需要安装 gh CLI 并运行 gh auth login")


def build() -> None:
    print("\n[1/5] 构建前端 …")
    if not DIST_DIR.exists():
        sys.exit(f"错误: 找不到前端目录 {GUI_DIR}")
    run(["pnpm", "build"], cwd=GUI_DIR)
    if not (DIST_DIR / "index.html").exists():
        sys.exit("错误: 构建失败，dist/index.html 未生成")


def pack(version: str) -> Path:
    print(f"\n[2/5] 打包 kaubo-v{version}.tar.gz …")
    tmpdir = Path(tempfile.mkdtemp())
    tarball = tmpdir / f"kaubo-v{version}.tar.gz"
    with tarfile.open(tarball, "w:gz") as tar:
        for item in sorted(DIST_DIR.iterdir()):
            tar.add(item, arcname=item.name)
    size_mb = os.path.getsize(tarball) / (1024 * 1024)
    print(f"      打包完成 ({size_mb:.1f} MB)")
    return tarball


def release(version: str, tarball: Path, skip_confirm: bool) -> None:
    print(f"\n[3/5] 发布到 GitHub Release v{version} …")
    tag = f"v{version}"

    confirm = "y" if skip_confirm else input(f"      确认发布 v{version}? [y/N] ")
    if confirm.lower() != "y":
        print("      已取消")
        sys.exit(0)

    run(["gh", "release", "create", tag,
         "--title", tag,
         "--notes", f"Kaubo Playground v{version}",
         str(tarball)])
    print(f"      已发布 → https://github.com/{_get_repo()}/releases/tag/{tag}")


def write_back_version(version: str) -> None:
    print(f"\n[4/5] 写入 .version → {version}")
    write_version(version)


def cleanup(tarball: Path) -> None:
    print(f"\n[5/5] 清理临时文件 …")
    if tarball.parent.exists():
        shutil.rmtree(tarball.parent)
    print("      完成")


def _get_repo() -> str:
    result = subprocess.run(
        ["gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"],
        cwd=REPO_ROOT, capture_output=True, text=True,
    )
    return result.stdout.strip() if result.returncode == 0 else "owner/repo"


def main() -> None:
    parser = argparse.ArgumentParser(description="Kaubo 发布脚本")
    parser.add_argument("version", nargs="?", help="直接指定版本号 (不 auto bump)")
    parser.add_argument("--bump", choices=["major", "minor", "patch"], default="patch",
                        help="自动升版本 (默认 patch)")
    parser.add_argument("-y", "--yes", action="store_true", help="跳过确认")
    args = parser.parse_args()

    check_prerequisites()

    if args.version:
        version = args.version
    else:
        old = read_version()
        version = bump_version(old, args.bump)
        print(f"版本号: {old} → {version}")

    build()
    tarball = pack(version)
    try:
        release(version, tarball, args.yes)
        write_back_version(version)
    finally:
        cleanup(tarball)


if __name__ == "__main__":
    main()
