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
    """控制台事件输出——带前缀的人类可读格式。"""

    def emit(self, level: str, message: str) -> None:
        prefix = {
            "step": "\n[step]",
            "info": "  →",
            "error": "  ✗",
            "success": "  ✓",
        }.get(level, "   ")
        stream = sys.stderr if level == "error" else sys.stdout
        print(f"{prefix} {message}", file=stream)
