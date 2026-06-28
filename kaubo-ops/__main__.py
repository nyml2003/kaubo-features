"""kaubo-ops 入口——使得 `python kaubo-ops <cmd>` 可用。

用法:
    cd packages/kaubo-features
    python kaubo-ops ci
    python kaubo-ops build-wasm
    python kaubo-ops dev
"""

import sys
from pathlib import Path

# 把 kaubo-ops/ 自身加入 sys.path，使 cli/ app/ domain/ infra/ 可被 import
_ops_root = Path(__file__).resolve().parent
if str(_ops_root) not in sys.path:
    sys.path.insert(0, str(_ops_root))

from cli.main import main

if __name__ == "__main__":
    sys.exit(main())
