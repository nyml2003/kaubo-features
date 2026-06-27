# 词法

## 注释

支持行注释和块注释：

```kaubo
// line comment

/* block
   comment */
```

注释会被 lexer 识别，parser 在语句边界会跳过注释。

## 标识符

标识符以 ASCII 字母或 `_` 开头，后续可以包含字母、数字或 `_`：

```kaubo
value
add_one
Point
_tmp
```

当前 lexer 的首字符分支只接受 ASCII 字母和 `_`。后续字符使用 `is_alphanumeric()`，但建议文档和示例先保持 ASCII。

## 关键字

核心 lexer 当前识别这些关键字：

```text
const var if else for in while break continue return
struct impl match export import from as async await self
true false null not and or
```

VSCode grammar 中出现的 `elif`、`pass`、`yield`、`module`、`pub`、`operator`、`json` 等不是当前核心 parser 关键字。

## 字符串

字符串支持双引号和单引号：

```kaubo
"hello"
'hello'
```

当前转义包括：

```text
\n \r \t \\ \" \'
```

未闭合字符串会产生词法错误 token。

## 模板字符串

反引号 `` ` `` 界定模板字符串，`{expr}` 内嵌表达式：

```kaubo
const msg = `hello {name}, age {age}`;
```

lexer 将反引号内的完整内容作为一个 `TemplateString` token 产出，
parser 负责解析 `{...}` 段并脱糖为字符串拼接链。

转义支持：`` \` `` `` \n `` `` \t `` `` \\ ``。

## 数字

整数：

```kaubo
42
1_000
```

浮点数：

```kaubo
1.5
42.0
```

lexer 会把 `42.as_float()` 识别为整数 `42`、点号和标识符，而不是浮点数。

## 运算符和分隔符

当前 lexer 识别：

```text
+ - * / %
= == != < <= > >=
not and or
| |> -> >>
? ?. ?[ ?? ... `
( ) { } [ ] , ; : .
```

不是所有已识别运算符都有完整 lowering/runtime 支持。详见 [表达式](04-expressions.md) 和 [部分实现的语法表面](10-partial-features.md)。
