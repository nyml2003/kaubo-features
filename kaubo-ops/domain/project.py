"""聚合根：KauboProject——整个 Kaubo 工程的完整模型。

所有路径映射和制品消费关系只在此处维护。
改目录结构只改 config.json，所有用例自动生效。
"""

import json
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class KauboProject:
    """聚合根：Kaubo 项目的完整工程模型。"""

    root: Path

    # 子项目路径 —— 由 config.json 填充
    rust_workspace: Path = field(init=False)
    cli_crate: Path = field(init=False)
    wasm_crate: Path = field(init=False)
    gui_root: Path = field(init=False)
    gui_app_dir: Path = field(init=False)
    gui_types_dir: Path = field(init=False)
    gui_wasm_pkg: Path = field(init=False)
    vscode_root: Path = field(init=False)
    vscode_wasm_dir: Path = field(init=False)
    version_file: Path = field(init=False)
    gui_dist: Path = field(init=False)
    nginx_conf_src: Path = field(init=False)
    benchmark_suites_dir: Path = field(init=False)

    # WASM 配置
    wasm_out_name: str = field(init=False)
    wasm_targets: list[dict] = field(init=False)

    # Release 配置
    bump_default: str = field(init=False)

    def __post_init__(self):
        config = self._load_config()

        paths = config["paths"]
        self.rust_workspace = self.root / paths["rust_workspace"]
        self.cli_crate = self.root / paths["cli_crate"]
        self.wasm_crate = self.root / paths["wasm_crate"]
        self.gui_root = self.root / paths["gui_root"]
        self.gui_app_dir = self.root / paths["gui_app_dir"]
        self.gui_types_dir = self.root / paths["gui_types_dir"]
        self.gui_wasm_pkg = self.root / paths["gui_wasm_pkg"]
        self.vscode_root = self.root / paths["vscode_root"]
        self.vscode_wasm_dir = self.root / paths["vscode_wasm_dir"]
        self.version_file = self.root / paths["version_file"]
        self.gui_dist = self.root / paths["gui_dist"]
        self.nginx_conf_src = self.root / paths["nginx_conf_src"]
        self.benchmark_suites_dir = self.root / paths["benchmark_suites_dir"]

        wasm = config["wasm"]
        self.wasm_out_name = wasm["out_name"]
        self.wasm_targets = wasm["targets"]

        self.bump_default = config["release"]["bump_default"]

    def _load_config(self) -> dict:
        config_path = Path(__file__).resolve().parent.parent / "config.json"
        if not config_path.exists():
            raise FileNotFoundError(f"配置文件不存在: {config_path}")
        return json.loads(config_path.read_text(encoding="utf-8"))

    # ── 工厂方法 ──────────────────────────────────────────────

    def create_wasm_artifacts(self) -> list["WasmArtifact"]:
        """WASM 构建的两个目标产物。"""
        from domain.wasm import WasmArtifact, WasmTarget
        return [
            WasmArtifact(
                crate=self.wasm_crate,
                target=WasmTarget(t["target"]),
                output_dir=self.root / t["output"],
                consumer=t["consumer"],
                out_name=self.wasm_out_name,
            )
            for t in self.wasm_targets
        ]

    def create_rust_workspace(self) -> "RustWorkspace":
        from domain.rust import RustWorkspace
        return RustWorkspace(root=self.rust_workspace)

    def create_gui_app(self) -> "GuiApp":
        from domain.gui import GuiApp
        return GuiApp(
            root=self.gui_app_dir,
            workspace_root=self.gui_root,
            types_dir=self.gui_types_dir,
            wasm_pkg=self.gui_wasm_pkg,
        )

    def create_vscode_extension(self) -> "VscodeExtension":
        from domain.vscode import VscodeExtension
        return VscodeExtension(root=self.vscode_root, wasm_dir=self.vscode_wasm_dir)
