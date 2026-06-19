# 模块语法

## 当前状态

模块语法目前主要是 parser/AST 表面。driver 主路径仍处理单文件源码，不做模块路径解析、文件加载、链接、命名空间隔离或导出表。

也就是说，下面语法可以被 parser 接受，但不代表已经有完整模块系统。

## import path

```kaubo
import "std/prelude";
```

AST 会记录 path，`names` 为空，`alias` 为空。

## import alias

```kaubo
import "math" as math;
```

AST 会记录 path 和 alias。

## named import

当前 parser 支持的是：

```kaubo
import { sqrt, sin } from "std/math";
```

注意：VSCode snippet 中可能出现 `from "module" import { names };` 形态，但核心 parser 当前接受的是 `import { names } from "path";`。

## export

```kaubo
export const answer = 42;
```

`export` 会包住后续顶层语句形成 AST 节点。

## 当前限制

- `import` 在 infer 中跳过，在 lowering 中也不会加载外部文件。
- `export` 不会生成导出表。
- 没有 `module` 声明语法。
- 没有包解析、相对路径规则、循环依赖处理或可见性规则。

在实现完整模块系统前，模块语法应文档化为 parse-only。
