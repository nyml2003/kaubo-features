"""部署用例——从 GitHub Release 下载并部署到 nginx。

从 next_kaubo/ops/deploy/deploy.py 迁移逻辑。

前提：纯 Python3 stdlib，无 pip 依赖。
      nginx 已安装且有权限执行 nginx -s reload。
"""

import json
import os
import shutil
import subprocess
import sys
import tarfile
import urllib.request
from pathlib import Path

from domain.project import KauboProject
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class DeployApp:
    """部署用例——从 GitHub Release 下载产物，部署到 nginx。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject, version: str | None = None,
            deploy_root: Path | None = None, nginx_conf: Path | None = None,
            repo: str | None = None) -> bool:

        version = version or self._read_version(project.version_file)
        deploy_root = deploy_root or Path(os.environ.get("DEPLOY_ROOT", "/var/www/kaubo"))
        nginx_conf = nginx_conf or Path(os.environ.get("NGINX_CONF", "/etc/nginx/conf.d/kaubo.conf"))
        repo = repo or os.environ.get("KAUBO_REPO", "nyml2003/kaubo-features")
        dl_mirror = os.environ.get("DEPLOY_MIRROR", "https://ghfast.top/")

        # 前置检查
        if shutil.which("nginx") is None:
            self.events.emit("error", "nginx is not installed")
            return False

        # 跳过已部署版本
        if self._check_skip(version, deploy_root):
            return True

        # 获取下载 URL
        self.events.emit("step", f"Fetching release info for v{version}")
        download_url = self._get_download_url(repo, version)

        # 部署
        dist_dir = deploy_root / "dist"
        tag_file = deploy_root / ".deployed_version"

        # 安装 nginx 配置
        self.events.emit("step", "Installing nginx config")
        self._install_nginx_conf(project.nginx_conf_src, nginx_conf)

        # 清空部署目录
        self.events.emit("step", "Clearing deploy directory")
        dist_dir.mkdir(parents=True, exist_ok=True)
        for item in list(dist_dir.iterdir()):
            if item.is_dir():
                shutil.rmtree(item)
            else:
                item.unlink()
        self.events.emit("info", f"  {dist_dir} cleared")

        # 下载 + 解压
        self.events.emit("step", f"Downloading and extracting v{version}")
        try:
            mirror_url = dl_mirror + download_url
            req = urllib.request.Request(mirror_url)
            req.add_header("User-Agent", "kaubo-deploy")
            with urllib.request.urlopen(req) as resp:
                with tarfile.open(fileobj=resp, mode="r:gz") as tar:
                    tar.extractall(path=dist_dir)
        except Exception as e:
            self.events.emit("error", f"Download/extract failed: {e}")
            return False

        file_count = sum(1 for _ in dist_dir.iterdir())
        self.events.emit("info", f"  Extracted ({file_count} files)")

        # nginx 重载
        self.events.emit("step", "Reloading nginx")
        r = subprocess.run(["nginx", "-t"], capture_output=True, text=True)
        if r.returncode != 0:
            self.events.emit("error", f"nginx config check failed:\n{r.stderr}")
            return False
        r = subprocess.run(["nginx", "-s", "reload"], capture_output=True, text=True)
        if r.returncode != 0:
            self.events.emit("error", f"nginx reload failed: {r.stderr}")
            return False
        self.events.emit("info", "  nginx reloaded")

        tag_file.write_text(version)
        self.events.emit("success", f"Deployed v{version}")
        return True

    def _read_version(self, version_file: Path) -> str:
        if not version_file.exists():
            sys.exit(f"Error: version file not found: {version_file}")
        return version_file.read_text().strip()

    def _check_skip(self, version: str, deploy_root: Path) -> bool:
        tag_file = deploy_root / ".deployed_version"
        if tag_file.exists():
            current = tag_file.read_text().strip()
            if current == version:
                self.events.emit("info", f"Already up to date (v{version})")
                return True
        return False

    def _get_download_url(self, repo: str, version: str) -> str:
        url = f"https://api.github.com/repos/{repo}/releases"
        try:
            req = urllib.request.Request(url)
            req.add_header("Accept", "application/vnd.github+json")
            req.add_header("User-Agent", "kaubo-deploy")
            with urllib.request.urlopen(req) as resp:
                releases = json.loads(resp.read())
        except urllib.error.HTTPError as e:
            sys.exit(f"Error: HTTP {e.code} - {e.reason}")
        except Exception as e:
            sys.exit(f"Error: GitHub API failed - {e}")

        if not isinstance(releases, list) or not releases:
            sys.exit(f"Error: repo {repo} has no releases")

        for rel in releases:
            tag = rel.get("tag_name", "")
            name = rel.get("name", "")
            if version in tag or version in name:
                assets = rel.get("assets", [])
                if not assets:
                    sys.exit(f"Error: Release {tag} has no assets")
                return assets[0]["browser_download_url"]

        sys.exit(f"Error: version {version} not found in repo {repo}")

    def _install_nginx_conf(self, src: Path, target: Path) -> bool:
        if not src.exists():
            self.events.emit("info", "  (nginx config source not found, skipping)")
            return False

        new_content = src.read_bytes()
        old_content = target.read_bytes() if target.exists() else b""

        if old_content == new_content:
            self.events.emit("info", "  nginx config unchanged, skipping")
            return False

        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(new_content)
        self.events.emit("info", f"  nginx config updated → {target}")
        return True
