# 内置函数与方法

## 输出

| 函数 | 签名 | 说明 |
|------|------|------|
| `print` | `print val` | 输出到 stdout |

```kaubo
print "Hello, World!";
print 42;
print(true);
print(sqrt(16));
```

## 类型

| 函数 | 签名 | 说明 |
|------|------|------|
| `type` | `type(val) → string` | 返回值的类型名 |
| `to_string` | `to_string(val) → string` | 将值转为字符串 |

```kaubo
print type(42);      // "int"
print type("hello");  // "string"
```

## 数学

| 函数 | 签名 | 说明 |
|------|------|------|
| `sqrt` | `sqrt(x: float) → float` | 平方根 |
| `sin` | `sin(x: float) → float` | 正弦 |
| `cos` | `cos(x: float) → float` | 余弦 |
| `floor` | `floor(x: float) → int` | 向下取整 |
| `ceil` | `ceil(x: float) → int` | 向上取整 |

```kaubo
print sqrt(16);   // 4
print sin(0);      // 0
print cos(0);      // 1
print floor(3.14); // 3
print ceil(3.14);  // 4
```

## 列表

| 函数/方法 | 签名 | 说明 |
|-----------|------|------|
| `len` | `list.len() → int` | 列表长度 |
| `push` | `list.push(val) → List` | 末尾添加，返回自身（可链式） |

| 方法 | 签名 | 说明 |
|------|------|------|
| `.len()` | `() → int` | 列表长度 |
| `.push(val)` | `(val) → List` | 末尾添加 |
| `.map(fn)` | `(|x|→U) → List<U>` | 映射 |
| `.filter(fn)` | `(|x|→bool) → List` | 过滤 |
| `.find(fn)` | `(|x|→bool) → T` | 查找第一个匹配 |
| `.reduce(fn, init)` | `(|acc,x|→T, T) → T` | 归约 |
| `.foreach(fn)` | `(|x|→void) → void` | 遍历 |

```kaubo
var list = [1, 2, 3];

list.push(4).push(5);
print list.len();                  // 5

var doubled = list.map(|x| { return x * 2; });
print doubled.len();               // 5

var evens = list.filter(|x| { return x % 2 == 0; });
print evens.len();                 // 2
```

## 字符串

| 函数 | 签名 | 说明 |
|------|------|------|
| `length` | `length(str) → int` | 字符串长度 |
| `to_upper` | `to_upper(str) → string` | 转大写 |
| `to_lower` | `to_lower(str) → string` | 转小写 |
| `trim` | `trim(str) → string` | 去首尾空白 |
| `substring` | `substring(str, start, end) → string` | 子串 |
| `contains` | `contains(str, substr) → bool` | 是否包含 |
| `starts_with` | `starts_with(str, prefix) → bool` | 是否以 prefix 开头 |
| `ends_with` | `ends_with(str, suffix) → bool` | 是否以 suffix 结尾 |
| `split` | `split(str, delim) → List<string>` | 分割 |
| `join` | `join(list, delim) → string` | 连接 |
| `replace` | `replace(str, old, new) → string` | 替换 |

```kaubo
var msg = "hello kaubo";
print to_upper(msg);          // "HELLO KAUBO"
print length(msg);            // 12
print contains(msg, "kaub");  // true
```

## 工具

| 函数 | 签名 | 说明 |
|------|------|------|
| `range` | `range(start, end, [step]) → List<int>` | 生成整数列表 |
| `clone` | `clone(val) → T` | 浅拷贝 |
| `random` | `random() → float` | [0, 1) 随机浮点 |
| `random_int` | `random_int(min, max) → int` | [min, max] 随机整数 |
| `is_empty` | `is_empty(val) → bool` | 是否为空 |

## 协程

| 函数 | 签名 | 说明 |
|------|------|------|
| `create_coroutine` | `create_coroutine(fn) → Coroutine` | 创建协程 |
| `resume` | `resume(co) → T` | 恢复协程执行 |
| `coroutine_status` | `coroutine_status(co) → string` | 协程状态（待实现） |

```kaubo
var coro = create_coroutine(|| {
    print "start";
    yield 1;
    print "after yield";
    yield 2;
});

var r1 = resume(coro);  // start, 返回 1
var r2 = resume(coro);  // after yield, 返回 2
```

## 当前限制

以下函数已在 stdlib 中注册，但在 WASM/浏览器环境中不可用或不稳定：

| 函数 | 原因 |
|------|------|
| `read_file`, `write_file`, `exists`, `is_file`, `is_dir`, `create_dir`, `remove_file`, `rename` | 浏览器无文件系统 |
| `http_get`, `http_post` | 需要平台注入 |
| `sha256`, `base64_encode`, `base64_decode` | 未验证 |
| `now_timestamp`, `format_time` | 未验证 |
| `assert` | 不可用 |
| `PI`, `E` | 不可用 |
| `coroutine_status` | VM handler 未实现 |
