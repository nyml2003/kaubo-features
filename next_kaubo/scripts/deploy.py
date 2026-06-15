#! /usr/bin/env python3
"""Kaubo 部署脚本 —— 从 GitHub Release 下载并部署到 nginx。

用法:
    python3 scripts/deploy.py             # 读 .version，部署当前版本
    python3 scripts/deploy.py 0.1.0       # 部署/回滚到指定版本
    python3 deploy.py --repo owner/repo   # 指定 GitHub 仓库

前提: 纯 Python3 stdlib，无 pip 依赖。
      nginx 已安装且有权限执行 nginx -s reload。
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tarfile
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
DEPLOY_ROOT = Path(os.environ.get("DEPLOY_ROOT", "/var/www/kaubo"))
DIST_DIR = DEPLOY_ROOT / "dist"
DEPLOYED_TAG_FILE = DEPLOY_ROOT / ".deployed_version"
DEFAULT_REPO = os.environ.get("KAUBO_REPO", "nyml2003/kaubo-features")
DL_MIRROR   = os.environ.get("DEPLOY_MIRROR", "https://ghproxy.com/")


def read_version() -> str:
    version_file = REPO_ROOT / ".version"
    if not version_file.exists():
        sys.exit("错误: 找不到 .version 文件")
    version = version_file.read_text().strip()
    if not version:
        sys.exit("错误: .version 为空")
    return version


def get_download_url(repo: str, version: str) -> tuple[str, str]:
    """返回 (tag, download_url)"""
    url = f"https://api.github.com/repos/{repo}/releases/tags/v{version}"
    try:
        req = urllib.request.Request(url)
        req.add_header("Accept", "application/vnd.github+json")
        req.add_header("User-Agent", "kaubo-deploy")
        with urllib.request.urlopen(req) as resp:
            data = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        if e.code == 404:
            sys.exit(f"错误: 找不到 Release v{version} (仓库 {repo})")
        sys.exit(f"错误: HTTP {e.code} - {e.reason}")
    except Exception as e:
        sys.exit(f"错误: 访问 GitHub API 失败 - {e}")

    assets = data.get("assets", [])
    if not assets:
        sys.exit(f"错误: Release v{version} 没有附件")

    asset = assets[0]
    return data["tag_name"], asset["browser_download_url"]


def check_nginx() -> None:
    if shutil.which("nginx") is None:
        sys.exit("错误: 未安装 nginx")


def check_skip(version: str) -> bool:
    if DEPLOYED_TAG_FILE.exists():
        current = DEPLOYED_TAG_FILE.read_text().strip()
        if current == version:
            print(f"Already up to date (v{version})")
            return True
    return False


def do_deploy(version: str, download_url: str) -> None:
    # Step 1: 清空
    print(f"\n[1/3] 清空部署目录 …")
    DIST_DIR.mkdir(parents=True, exist_ok=True)
    for item in DIST_DIR.iterdir():
        if item.is_dir():
            shutil.rmtree(item)
        else:
            item.unlink()
    print(f"      {DIST_DIR} 已清空")

    # Step 2: 下载 + 解压
    print(f"\n[2/3] 下载 v{version} 并解压 …")
    try:
        mirror_url = DL_MIRROR + download_url
        req = urllib.request.Request(mirror_url)
        req.add_header("User-Agent", "kaubo-deploy")
        with urllib.request.urlopen(req) as resp:
            with tarfile.open(fileobj=resp, mode="r:gz") as tar:
                tar.extractall(path=DIST_DIR)
    except Exception as e:
        sys.exit(f"错误: 下载或解压失败 - {e}")

    file_count = sum(1 for _ in DIST_DIR.iterdir())
    print(f"      解压完成 ({file_count} 个文件)")

    # Step 3: nginx 重载
    print(f"\n[3/3] 重载 nginx …")
    result = subprocess.run(["nginx", "-t"], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"      !!! nginx 配置检查失败:\n{result.stderr}")
        sys.exit(1)
    result = subprocess.run(["nginx", "-s", "reload"], capture_output=True, text=True)
    if result.returncode != 0:
        sys.exit(f"错误: nginx reload 失败 - {result.stderr}")
    print("      nginx 已重载")

    # 记录部署版本
    DEPLOYED_TAG_FILE.write_text(version)
    print(f"\n部署完成: v{version}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Kaubo 部署脚本")
    parser.add_argument("version", nargs="?", help="版本号 (默认读 .version)")
    parser.add_argument("--repo", default=DEFAULT_REPO, help=f"GitHub 仓库 (默认 {DEFAULT_REPO})")
    args = parser.parse_args()

    version = args.version or read_version()
    check_nginx()

    if check_skip(version):
        return

    tag, download_url = get_download_url(args.repo, version)
    do_deploy(tag.lstrip("v"), download_url)


if __name__ == "__main__":
    main()
