"""WASM 构建产物——知道自己怎么构建、产到哪里、被谁消费。"""

from dataclasses import dataclass
from enum import Enum
from pathlib import Path


class WasmTarget(Enum):
    WEB = "web"
    NODEJS = "nodejs"


@dataclass
class WasmArtifact:
    """WASM 构建产物——一次构建，多目标产出。"""

    crate: Path
    target: WasmTarget
    output_dir: Path
    consumer: str          # 人类可读的消费者名称，用于日志
    out_name: str = "kaubo_wasm"

    def build_command(self) -> list[str]:
        return [
            "wasm-pack", "build", str(self.crate),
            "--target", self.target.value,
            "--out-dir", str(self.output_dir),
            "--out-name", self.out_name,
        ]
