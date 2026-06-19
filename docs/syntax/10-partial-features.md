# 部分实现的语法表面

本页列出已经出现在 lexer/parser/AST/infer 或编辑器侧，但不能按完整语言能力使用的特性。

## List literal

parser 和 infer 支持：

```kaubo
[1, 2, 3]
```

但 lowering 当前对非空 list 返回 `list literals are not implemented`。空 list 也不应视为稳定集合能力。

## Index

parser 和 infer 有 index 表面：

```kaubo
xs[0]
s[0]
```

VM 对 index 相关指令仍有未实现路径，index assignment 明确未实现。

## for

```kaubo
for x in xs {
  print(x);
}
```

parser/infer 表面存在，但 lowering 明确未实现。

## and / or

lexer、parser 和 infer 接受：

```kaubo
a and b
a or b
```

lowering 当前返回 `logical binary operators are not implemented`。

## Pipe 和 >>

parser 接受：

```kaubo
value |> f
f >> g
```

infer 目前近似 pass-through，lowering 返回 `pipe operators are not implemented`。不要当作已实现组合语义。

## async / await

parser 和 infer 接受：

```kaubo
async |id| { await f(id) }
```

lowering 当前会直接编译内部表达式，不代表完整 async runtime 语义已经接入语言主路径。

## string concatenation

AST 和 infer 中存在 `SAdd` 表面，但普通源码 parser 当前没有独立字符串拼接 token；lowering 对 string concatenation 返回未实现。

## 编辑器侧规划词

VSCode grammar/snippets 里可能出现：

```text
elif pass yield module pub operator json
```

这些不是当前核心语言主路径。新增或依赖这些语法前，应先补 lexer/parser/infer/lowering/runtime 的对应测试。
