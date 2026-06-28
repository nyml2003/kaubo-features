# 标准库和内建方法

## 当前状态

标准库函数由 VM native registry 提供，同时 infer 会注入一组内建签名（虚拟 prelude）。Phase 4a 之后新增了 9 个内置 interface 和 40+ 内置方法 impl。

不是所有有类型签名的名字都已经可运行。

## 内建 Interface（Phase 4a）

以下 interface 在编译期自动注入，用户可直接使用，无需显式声明：

`Add`、`Subtract`、`Multiply`、`Divide`、`Modulo`、`Compare`、`Display`、`IntoFloat`、`IntoInt`

这意味着 `a + b` 会自动查找 `Add` interface 对应的 impl，用户的 `impl Add for Vec2` 直接可用。

## 内建方法

以下方法在 `Int64`、`Float64`、`String`、`Bool` 上自动可用：

- **算术**：`add`、`subtract`、`multiply`、`divide`、`modulo`
- **比较**：`less`、`less_equal`、`greater`、`greater_equal`、`equal`、`not_equal`
- **转换**：`to_string`、`to_float`、`to_int`

## print

```kaubo
print("hi");
print(42.to_string());
```

`print` 会把输出交给 VM 捕获。当前 infer 签名偏向 `String -> Null`，但 lowering/VM 对非字符串值也有部分支持路径；稳定写法是打印字符串或显式 `.to_string()`。

## assert

```kaubo
assert(x > 0);
```

`assert` 接收布尔条件。条件为 false 时 native 返回运行错误。

## Math 函数

当前 registry 包含：

```text
sqrt
sin
cos
floor
ceil
```

这些函数按 `Float64 -> Float64` 使用：

```kaubo
sqrt(25.0);
floor(1.5);
```

整数需要先转浮点：

```kaubo
sqrt(25.to_float());
```

## to_string

内建 member 方法：

```kaubo
42.to_string()
1.5.to_string()
```

lowering 会分别生成整数到字符串、浮点到字符串的转换。

## to_float / to_int

内建 member 方法：

```kaubo
42.to_float()
1.5.to_int()
```

## type_of

`type_of` 在 infer 中有占位签名：

```text
forall a. a -> String
```

但 lowering 明确返回 `type_of is not implemented`，VM native 也返回未实现错误。当前不能把它当作可运行标准库能力。

## 不要从编辑器提示反推标准库

编辑器补全或 snippet 中出现的名字如果没有 infer、lowering 和 VM 覆盖，不应写入标准库语义。
