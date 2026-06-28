"""发布用例——自动升版本号、构建前端、发布到 GitHub Release。

从 next_kaubo/ops/release/publish.py 迁移逻辑，用领域对象替代裸字符串。

前提：安装了 pnpm / gh CLI (gh auth login)
"""

import os
import shutil
import sys
import tarfile
import tempfile
from pathlib import Path

from domain.project import KauboProject
from domain.version import ReleaseVersion
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


class PublishRelease:
    """发布用例——版本 bump + 构建 + GitHub Release。"""

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    def run(self, project: KauboProject, bump: str | None = None,
            version: str | None = None, skip_confirm: bool = False) -> bool:

        # 前置检查
        self._check_prerequisites()

        # 确定版本号
        if version:
            new_version = version
        else:
            old = self._read_version(project.version_file)
            old_ver = ReleaseVersion.parse(old)
            new_ver = old_ver.bump(bump or project.bump_default)
            new_version = str(new_ver)
            self.events.emit("info", f"Version: {old} → {new_version}")

        # 1. 构建前端
        self.events.emit("step", "Building frontend")
        gui = project.create_gui_app()
        self.runner.run(gui.types_build_command(), cwd=gui.types_dir)
        r = self.runner.run(gui.build_command(), cwd=gui.root)
        if not r.ok:
            self.events.emit("error", f"Frontend build failed:\n{r.stderr[-500:]}")
            return False

        dist_dir = project.gui_dist
        if not dist_dir.exists() or not (dist_dir / "index.html").exists():
            self.events.emit("error", f"Build failed: {dist_dir / 'index.html'} not found")
            return False

        # 2. 打包
        self.events.emit("step", f"Packing kaubo-v{new_version}.tar.gz")
        tarball = self._pack(dist_dir, new_version)
        size_mb = os.path.getsize(tarball) / (1024 * 1024)
        self.events.emit("info", f"  Packed ({size_mb:.1f} MB)")

        try:
            # 3. 发布
            self.events.emit("step", f"Release to GitHub v{new_version}")
            tag = f"v{new_version}"

            if not skip_confirm:
                confirm = input(f"       Confirm release v{new_version}? [y/N] ")
                if confirm.lower() != "y":
                    self.events.emit("info", "Cancelled")
                    return True

            r = self.runner.run(
                ["gh", "release", "create", tag,
                 "--title", tag,
                 "--notes", f"Kaubo Playground v{new_version}",
                 str(tarball)],
                cwd=project.root,
            )
            if not r.ok:
                self.events.emit("error", f"Release failed:\n{r.stderr[-500:]}")
                return False

            self.events.emit("success", f"Released → {tag}")

            # 4. 写回版本号
            self.events.emit("step", f"Writing .version → {new_version}")
            self.fs.write_text(project.version_file, new_version + "\n")

        finally:
            # 清理
            if tarball.parent.exists():
                shutil.rmtree(tarball.parent)

        return True

    def _check_prerequisites(self) -> None:
        if shutil.which("pnpm") is None:
            sys.exit("Error: pnpm is required (https://pnpm.io)")
        if shutil.which("gh") is None:
            sys.exit("Error: gh CLI is required — run 'gh auth login'")

    def _read_version(self, version_file: Path) -> str:
        if not version_file.exists():
            sys.exit(f"Error: version file not found: {version_file}")
        v = version_file.read_text().strip()
        if not v:
            sys.exit("Error: .version is empty")
        return v

    def _pack(self, dist_dir: Path, version: str) -> Path:
        tmpdir = Path(tempfile.mkdtemp())
        tarball = tmpdir / f"kaubo-v{version}.tar.gz"
        with tarfile.open(tarball, "w:gz") as tar:
            for item in sorted(dist_dir.iterdir()):
                tar.add(item, arcname=item.name)
        return tarball
