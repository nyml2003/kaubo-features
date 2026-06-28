# Kaubo 文档

Kaubo 是一个自研编程语言引擎。文档按读者分层组织：

## 快速导航

| 我想… | 入口 |
|--------|------|
| 了解 Kaubo 语法、写代码 | [语言参考](language/README.md) |
| 理解编译器架构、贡献代码 | [架构设计](architecture/README.md) |
| 跑 CI、测试、发布、部署 | [运维指南](operations/README.md) |
| 看整体规划方向和当前进度 | [路线图](roadmap.md) |

## 目录

```
docs/
├── README.md                   # ← 你在这里
├── roadmap.md                  # 路线图
│
├── language/                   # 语言参考（给 Kaubo 使用者）
│   ├── README.md               #   能力总览
│   ├── 01-lexical.md           #   词法
│   ├── 02-statements.md        #   语句
│   ├── 03-types.md             #   类型语法
│   ├── 04-expressions.md       #   表达式
│   ├── 05-functions.md         #   函数和 Lambda
│   ├── 06-control-flow.md      #   控制流
│   ├── 07-structs-and-impls.md #   Struct 和 Impl
│   ├── 08-modules.md           #   模块语法
│   ├── 09-builtins.md          #   标准库和内建方法
│   ├── 10-partial-features.md  #   部分实现
│   ├── 11-edge-cases.md        #   边界例子
│   └── xx-extensions.md        #   扩展特性状态
│
├── architecture/               # 架构设计（给编译器贡献者）
│   ├── README.md               #   管线全景 + crate 地图
│   ├── 01-parser.md            #   词法与语法分析
│   ├── 02-type-inference.md    #   类型推断
│   ├── 03-cps-ir.md            #   CPS IR：Lowering + Flatten + Passes
│   ├── 04-vm.md                #   寄存器 VM
│   ├── 05-module-system.md     #   模块系统
│   ├── 06-events-and-logging.md #  事件与日志系统
│   ├── 07-language-service.md  #   Language Service
│   └── 08-web-vscode.md         #  Web 和 VSCode 适配
│
└── operations/                 # 运维指南（给维护者）
    ├── README.md               #   运维总览
    ├── tooling.md              #   工具链（Ops2）
    ├── testing.md              #   测试指南
    └── deploy.md               #   发布和部署
```
