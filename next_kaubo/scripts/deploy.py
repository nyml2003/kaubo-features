#! /usr/bin/env python3
"""Kaubo 部署脚本 —— 从 GitHub Release 下载并部署到 nginx。

用法:
    python3 scripts/deploy.py                  # 读 .version，部署到 /var/www/kaubo/dist
    python3 scripts/deploy.py 0.1.0            # 部署指定版本
    python3 scripts/deploy.py --root /srv/web  # 指定部署目录
    python3 deploy.py --repo owner/repo        # 指定 GitHub 仓库

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
NGINX_CONF_SRC = REPO_ROOT / "nginx" / "kaubo.conf"
DEFAULT_DEPLOY_ROOT = Path(os.environ.get("DEPLOY_ROOT", "/var/www/kaubo"))
DEFAULT_NGINX_CONF = Path("/etc/nginx/conf.d/kaubo.conf")
DEFAULT_REPO = os.environ.get("KAUBO_REPO", "nyml2003/kaubo-features")
DL_MIRROR   = os.environ.get("DEPLOY_MIRROR", "https://ghfast.top/")


def read_version() -> str:
    version_file = REPO_ROOT / ".version"
    if not version_file.exists():
        sys.exit("错误: 找不到 .version 文件")
    version = version_file.read_text().strip()
    if not version:
        sys.exit("错误: .version 为空")
    return version


def get_download_url(repo: str, version: str) -> tuple[str, str]:
    """从 GitHub Releases 列表中找到指定版本，返回 (tag_name, download_url)。"""
    url = f"https://api.github.com/repos/{repo}/releases"
    try:
        req = urllib.request.Request(url)
        req.add_header("Accept", "application/vnd.github+json")
        req.add_header("User-Agent", "kaubo-deploy")
        with urllib.request.urlopen(req) as resp:
            releases = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        sys.exit(f"错误: HTTP {e.code} - {e.reason}")
    except Exception as e:
        sys.exit(f"错误: 访问 GitHub API 失败 - {e}")

    if not isinstance(releases, list) or not releases:
        sys.exit(f"错误: 仓库 {repo} 没有 Release")

    # 匹配 tag_name 或 name 中包含目标版本的 release
    for rel in releases:
        tag = rel.get("tag_name", "")
        name = rel.get("name", "")
        if version in tag or version in name:
            assets = rel.get("assets", [])
            if not assets:
                sys.exit(f"错误: Release {tag} 没有附件")
            return tag, assets[0]["browser_download_url"]

    sys.exit(f"错误: 找不到版本 {version} (仓库 {repo})")


def check_nginx() -> None:
    if shutil.which("nginx") is None:
        sys.exit("错误: 未安装 nginx")


def check_skip(version: str, deploy_root: Path) -> bool:
    tag_file = deploy_root / ".deployed_version"
    if tag_file.exists():
        current = tag_file.read_text().strip()
        if current == version:
            print(f"Already up to date (v{version})")
            return True
    return False


def install_nginx_conf(target: Path) -> bool:
    if not NGINX_CONF_SRC.exists():
        print("      (nginx 配置源文件不存在，跳过)")
        return False

    new_content = NGINX_CONF_SRC.read_bytes()
    old_content = target.read_bytes() if target.exists() else b""

    if old_content == new_content:
        print("      nginx 配置未变更，跳过")
        return False

    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(new_content)
    print(f"      nginx 配置已更新 → {target}")
    return True


def do_deploy(version: str, download_url: str, deploy_root: Path, nginx_conf: Path) -> None:
    dist_dir = deploy_root / "dist"
    tag_file = deploy_root / ".deployed_version"

    # Step 0: 安装 nginx 配置
    print(f"\n[0/4] 安装 nginx 配置 …")
    install_nginx_conf(nginx_conf)

    # Step 1: 清空
    print(f"\n[1/4] 清空部署目录 …")
    dist_dir.mkdir(parents=True, exist_ok=True)
    for item in dist_dir.iterdir():
        if item.is_dir():
            shutil.rmtree(item)
        else:
            item.unlink()
    print(f"      {dist_dir} 已清空")

    # Step 2: 下载 + 解压
    print(f"\n[2/4] 下载 v{version} 并解压 …")
    try:
        mirror_url = DL_MIRROR + download_url
        req = urllib.request.Request(mirror_url)
        req.add_header("User-Agent", "kaubo-deploy")
        with urllib.request.urlopen(req) as resp:
            with tarfile.open(fileobj=resp, mode="r:gz") as tar:
                tar.extractall(path=dist_dir)
    except Exception as e:
        sys.exit(f"错误: 下载或解压失败 - {e}")

    file_count = sum(1 for _ in dist_dir.iterdir())
    print(f"      解压完成 ({file_count} 个文件)")

    # Step 3: nginx 重载
    print(f"\n[3/4] 重载 nginx …")
    result = subprocess.run(["nginx", "-t"], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"      !!! nginx 配置检查失败:\n{result.stderr}")
        sys.exit(1)
    result = subprocess.run(["nginx", "-s", "reload"], capture_output=True, text=True)
    if result.returncode != 0:
        sys.exit(f"错误: nginx reload 失败 - {result.stderr}")
    print("      nginx 已重载")

    tag_file.write_text(version)
    print(f"\n部署完成: v{version}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Kaubo 部署脚本")
    parser.add_argument("version", nargs="?", help="版本号 (默认读 .version)")
    parser.add_argument("--root", default=DEFAULT_DEPLOY_ROOT, type=Path,
                        help=f"部署根目录 (默认 {DEFAULT_DEPLOY_ROOT})")
    parser.add_argument("--nginx-conf", default=DEFAULT_NGINX_CONF, type=Path,
                        help=f"nginx 配置目标路径 (默认 {DEFAULT_NGINX_CONF})")
    parser.add_argument("--repo", default=DEFAULT_REPO,
                        help=f"GitHub 仓库 (默认 {DEFAULT_REPO})")
    args = parser.parse_args()

    version = args.version or read_version()
    check_nginx()

    if check_skip(version, args.root):
        return

    tag, download_url = get_download_url(args.repo, version)
    do_deploy(tag.lstrip("v"), download_url, args.root, args.nginx_conf)


if __name__ == "__main__":
    main()
