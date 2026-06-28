# 运维指南

目标读者：维护 Kaubo 构建、测试、发布和部署流程的开发者。

所有操作入口统一为 `python kaubo-ops <cmd>`。

## 文档

| 文档 | 内容 |
|------|------|
| [tooling.md](tooling.md) | Ops2 工具链架构、全部 21 个命令、问题解决记录 |
| [testing.md](testing.md) | 测试分层策略、标准命令、Bug 修复流程 |
| [deploy.md](deploy.md) | 发布到 GitHub Release、部署到 nginx、Benchmark、覆盖率 |

## 常用命令速查

```bash
python kaubo-ops ci              # 标准 CI
python kaubo-ops check           # 快速类型检查
python kaubo-ops test            # 全部测试
python kaubo-ops test-rust       # Rust 测试
python kaubo-ops lint            # 全部 lint
python kaubo-ops fmt             # 全部格式化
python kaubo-ops dev             # 启动 Web 开发服务器
python kaubo-ops build           # 构建所有产物
python kaubo-ops release --bump patch  # 发布
```

## 项目总览

- [路线图](../roadmap.md)
- [架构设计](../architecture/README.md)
- [语言参考](../language/README.md)
