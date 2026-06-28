"""文件系统操作抽象——所有文件 I/O 的唯一入口。"""

import shutil
from abc import ABC, abstractmethod
from pathlib import Path


class FileSystem(ABC):
    """文件系统操作抽象。"""

    @abstractmethod
    def exists(self, path: Path) -> bool: ...

    @abstractmethod
    def read_text(self, path: Path) -> str: ...

    @abstractmethod
    def write_text(self, path: Path, content: str) -> None: ...

    @abstractmethod
    def mkdir_p(self, path: Path) -> None: ...

    @abstractmethod
    def rmtree(self, path: Path) -> None: ...


class RealFileSystem(FileSystem):
    """真实的文件系统操作——委托给 pathlib + shutil。"""

    def exists(self, path: Path) -> bool:
        return path.exists()

    def read_text(self, path: Path) -> str:
        return path.read_text(encoding="utf-8")

    def write_text(self, path: Path, content: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def mkdir_p(self, path: Path) -> None:
        path.mkdir(parents=True, exist_ok=True)

    def rmtree(self, path: Path) -> None:
        if path.exists():
            shutil.rmtree(path)
