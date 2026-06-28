"""工具版本和环境检查。"""

import shutil
import sys


def check_tools(tools: list[str]) -> list[str]:
    """检查哪些工具缺失，返回缺失列表。"""
    missing: list[str] = []
    for t in tools:
        if shutil.which(t) is None:
            missing.append(t)
    return missing


def require_tools(tools: list[str]) -> None:
    """检查必需工具，缺失时打印错误并退出。"""
    missing = check_tools(tools)
    if missing:
        for t in missing:
            print(f"错误: 未找到必需工具 — {t}", file=sys.stderr)
        print(f"\n请安装缺失的工具后重试。", file=sys.stderr)
        sys.exit(1)
