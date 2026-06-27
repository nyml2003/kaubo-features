# 控制流

## if / else

`if` 是表达式：

```kaubo
const x = if n < 0 { -n } else { n };
```

有 `else` 时，infer 会要求两边类型可统一。

没有 `else` 的形态也能 parse/build，但作为表达式值使用时应谨慎：

```kaubo
if ok { print("yes"); };
```

## while

`while` 是表达式表面，主路径可运行：

```kaubo
var n = 0;
while n < 3 {
  n = n + 1;
};
n;
```

`while` 的结果类型视为 `Null`。

## break / continue

`break` 和 `continue` 在 while lowering 中有跳转目标：

```kaubo
while true {
  break;
};
```

在循环外使用会返回 build 错误。

## match

`match` 是表达式，对值做多路分支：

```kaubo
const desc = match x {
    0 -> "zero",
    1 -> "one",
    _ -> "many",
};
```

- 每个 arm 使用 `->` 分隔模式和体
- `_` 是通配符，必须作为最后一个 arm
- 脱糖为 `if/else` 链：`{ var t = x; if t == 0 { "zero" } else if t == 1 { "one" } else { "many" } }`
- 对 enum 变体的匹配走 tag 比较（`GetVariantTag` 指令），而非值比较
- 模式解构暂未实现，当前仅支持常量匹配和通配符

```kaubo
enum Color { Red, Green };
const result = match c {
    Red -> 100,
    Green -> 200,
    _ -> 0,
};
```

## for

parser 和 infer 有 `for x in xs { ... }` 表面：

```kaubo
for x in xs {
  print(x);
}
```

但 lowering 明确返回 `for loops are not implemented in lowering`。当前不能把 `for` 当作可运行能力。
