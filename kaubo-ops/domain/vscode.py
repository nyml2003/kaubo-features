"""VSCode 扩展——纯静态资源，行为全部由 Ops2 定义。

对应的 package.json 无 scripts 字段（Phase 3 后生效）。
"""

from dataclasses import dataclass
from pathlib import Path


@dataclass
class VscodeExtension:
    """VSCode 扩展——命令的领域对象。"""

    root: Path
    wasm_dir: Path   # WASM 产物消费位置

    def test_command(self) -> list[str]:
        return ["node", "--test", "tests/*.test.js"]

    def package_command(self) -> list[str]:
        return ["pnpm", "exec", "vsce", "package"]
