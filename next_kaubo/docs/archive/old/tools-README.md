# Kaubo Tools

统一测试 / 基准 / 分析工具集。

## 命令

```bash
# 跑全部 benchmark
python tools/runner.py bench

# 跑单个 suite
python tools/runner.py bench --suite fib

# 只跑 kaubo
python tools/runner.py bench --lang kaubo

# 输出 JSON 报告
python tools/runner.py bench --json --output results/report.json

# 跑集成测试
python tools/runner.py test

# 性能分析 (需要 flamegraph)
python tools/runner.py profile --suite mandelbrot
```

## 添加 Benchmark

1. 在 `tools/bench/kaubo/` 下创建 `.kaubo` 文件
2. 在 `tools/bench/suites.toml` 注册配置
3. 可选：添加 Python/Rust 参考实现

```toml
[my_bench]
description = "My benchmark"
expected = "42"
iterations = 5
warmup = 1

[my_bench.kaubo]
file = "my_bench.kaubo"

[my_bench.python]
function = "my_func"
args = [10]
```

## 添加集成测试

在 `tools/test/examples.toml` 添加：

```toml
[[tests]]
name = "my_test"
entry = "examples/my_example/main.kaubo"
expected_output = "Hello"  # 可选
```

## 目录结构

```
tools/
├── runner.py              # 入口
├── lib/
│   ├── bench.py           # Benchmark 引擎
│   ├── test.py            # 集成测试引擎
│   ├── profile.py         # 性能分析
│   └── report.py          # 统一输出
├── bench/
│   ├── suites.toml        # Benchmark 声明
│   ├── kaubo/             # Kaubo 测试文件
│   ├── python/            # Python 参考实现
│   └── rust/              # Rust 参考实现
└── test/
    └── examples.toml      # 集成测试声明
```
