"""事件/日志输出抽象——所有用户可见输出的唯一入口。

CLI 输出、CI 日志、结构化文件都通过此抽象。
"""

import sys
from abc import ABC, abstractmethod


class EventBus(ABC):
    """进度/事件输出抽象。"""

    @abstractmethod
    def emit(self, level: str, message: str) -> None:
        ...


class ConsoleEventBus(EventBus):
    """控制台事件输出——带前缀的人类可读格式。

    前缀使用 ASCII 安全字符，避免 Windows GBK 终端编码问题。
    """

    # 使用 ASCII 安全前缀，兼容所有终端编码
    PREFIXES = {
        "step": "\n[step]",
        "info": "  >",
        "error": "  X",
        "success": "  OK",
    }

    def emit(self, level: str, message: str) -> None:
        prefix = self.PREFIXES.get(level, "   ")
        stream = sys.stderr if level == "error" else sys.stdout
        # 用 errors='replace' 防止非 ASCII 字符导致 UnicodeEncodeError
        safe = f"{prefix} {message}"
        try:
            print(safe, file=stream)
        except UnicodeEncodeError:
            print(safe.encode(stream.encoding or "utf-8", errors="replace").decode(stream.encoding or "utf-8", errors="replace"), file=stream)
