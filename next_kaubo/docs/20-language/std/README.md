# Kaubo 标准库 (MVP)

> Kaubo 内置的标准库函数和模块。版本：v0.1.0

---

## 使用

```kaubo
import std;
sqrt(16.0);
print("hello");
```

---

## 核心函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `print` | `(any) -> void` | 打印到 stdout |
| `assert` | `(bool, string?) -> void` | 断言，可选消息 |
| `type` | `(any) -> string` | 获取类型名 |
| `to_string` | `(any) -> string` | 转为字符串 |

## 数学

| 函数 | 签名 | 说明 |
|------|------|------|
| `sqrt` | `(float) -> float` | 平方根 |
| `sin` | `(float) -> float` | 正弦 |
| `cos` | `(float) -> float` | 余弦 |
| `floor` | `(float) -> float` | 向下取整 |
| `ceil` | `(float) -> float` | 向上取整 |
| `PI` | `float` | 圆周率 |
| `E` | `float` | 自然对数底 |

## 容器操作

| 函数 | 签名 | 说明 |
|------|------|------|
| `len` | `(any) -> int` | 长度（list/string/json） |
| `push` | `(list, any) -> list` | 追加（返回新列表） |
| `is_empty` | `(any) -> bool` | 判空（list/string/json） |
| `range` | `(int, int?, int?) -> List[int]` | 生成整数范围 (1-3 args) |
| `clone` | `(any) -> any` | 浅拷贝 |

## 文件 I/O

| 函数 | 签名 | 说明 |
|------|------|------|
| `read_file` | `(string) -> string` | 读取文件 |
| `write_file` | `(string, string) -> void` | 写入文件 |
| `exists` | `(string) -> bool` | 路径是否存在 |
| `is_file` | `(string) -> bool` | 是否为文件 |
| `is_dir` | `(string) -> bool` | 是否为目录 |

## 字符串

| 函数 | 签名 | 说明 |
|------|------|------|
| `substring` | `(string, int, int) -> string` | 子串 |
| `contains` | `(string, string) -> bool` | 包含 |
| `starts_with` | `(string, string) -> bool` | 前缀匹配 |
| `ends_with` | `(string, string) -> bool` | 后缀匹配 |

## 环境与时间

| 函数 | 签名 | 说明 |
|------|------|------|
| `env` | `(string) -> string` | 环境变量 |
| `now` | `() -> float` | Unix 时间戳 |

## 协程

| 函数 | 签名 | 说明 |
|------|------|------|
| `create_coroutine` | `(closure) -> coroutine` | 创建协程 |
| `resume` | `(coroutine, ...args) -> any` | 恢复执行 |
| `coroutine_status` | `(coroutine) -> int` | 返回 0/1/2 |

---

## 内置方法

### List

| 方法 | 说明 |
|------|------|
| `push(x)` | 追加 |
| `len()` | 长度 |
| `remove(i)` | 按索引删除 |
| `clear()` | 清空 |
| `is_empty()` | 判空 |
| `foreach(\|x\| ...)` | 遍历 |
| `map(\|x\| ...)` | 映射 |
| `filter(\|x\| ...)` | 过滤 |
| `reduce(\|acc, x\| ..., init)` | 归约 |
| `find(\|x\| ...)` | 查找 |
| `any(\|x\| ...)` | 任一满足 |
| `all(\|x\| ...)` | 全部满足 |

### String

| 方法 | 说明 |
|------|------|
| `len()` | 长度 |
| `is_empty()` | 判空 |

### JSON

| 方法 | 说明 |
|------|------|
| `len()` | 属性数量 |
| `is_empty()` | 判空 |

---

*最后更新：2026-06-11*
