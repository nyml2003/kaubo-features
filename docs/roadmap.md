# 路线图

目标读者：规划架构和功能工作的维护者。

## 当前方向

长期核心任务是用共享编译器事实替代散落在各层的 heuristic。

现在 Web 高亮、language service 补全、lowering 和 VM 行为都可能依赖局部假设。目标状态是 syntax、semantic facts、lowering 和 adapters 之间都有清晰契约。

## Phase 1：保留源码事实

给 diagnostics 和编辑器能力关心的语法节点补可靠 spans：

- declarations；
- variable references；
- type references；
- struct names；
- field names；
- method names；
- member access；
- call expressions。

这一阶段应保持 parser 和 AST 职责窄：保留源码结构和范围，不嵌入 Web-specific 逻辑。

## Phase 2：构建语义事实

引入共享 semantic model，用来回答：

- 声明了哪些 symbols；
- 有哪些 scopes；
- reference 解析到哪里；
- expression 的类型是什么；
- 某个值上有哪些 fields 和 methods；
- unresolved names 或 invalid members 应该报告什么 diagnostics。

这可以先从 `kaubo-language-service` 内部开始，但最终目标是一个能被 editor adapters 之外复用的 compiler-facing semantic layer。

## Phase 3：让编辑器能力消费事实

让 Web、VSCode 和未来 CLI 高亮消费同一套 service DTO：

- semantic tokens；
- completions；
- hover；
- go to definition；
- diagnostics。

适配层只负责把 DTO 转成宿主 API。

## Phase 4：让 Lowering 消费事实

Lowering 应该消费类型和 member-resolution facts，而不是根据名字或局部 hint 猜测。

重要结果：

- field access 按 object type 解析；
- method call 按 receiver type 解析；
- struct literal 按声明字段 index 写入；
- invalid names 在 VM 执行前失败；
- int/float/string conversions 根据已知类型选择。

## Phase 5：收紧 VM 契约

让 runtime value contract 显式化：

- 避免 silent fallback values；
- 对非法操作返回 runtime errors；
- 记录并测试 int/float/heap payload 规则；
- 只有在测试描述清楚当前行为和目标替代行为之后，再考虑 tagged value model。

开始时不需要完整重写。第一步收益最大的是移除 silent no-op 和 `0` fallback 行为。

## 优先级建议

当 highlighter bug 和 interpreter bug 同时出现时，优先补共享事实和可执行回归，而不是只修 adapter 层。局部 Web-only 修复也许能短期改善颜色，但不能解决 completion、VSCode 和 runtime 之间的语义不一致。
