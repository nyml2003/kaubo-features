# 发布和部署

目标读者：维护 Web Playground 发布、部署和运维入口的开发者。

## 当前状态

所有 CI/CD 入口统一收口到 `kaubo-ops`（Ops2），不再使用 `cargo-make` / `Makefile.toml`（已移除）。

```bash
python kaubo-ops release --bump patch    # 发布到 GitHub Release
python kaubo-ops deploy 0.5.0            # 部署到 nginx
```

## 发布 Release

发布脚本会读取或更新 `.version`，构建 Web app，把 `dist/` 打成 `kaubo-vX.Y.Z.tar.gz`，然后通过 `gh release create` 上传到 GitHub Release。

```bash
python kaubo-ops release --bump patch
python kaubo-ops release --bump minor
python kaubo-ops release --bump major
```

前提：

- 已安装 `pnpm` 和 GitHub CLI `gh`。
- 已执行 `gh auth login`。

## 部署到服务器

部署脚本从 GitHub Release 下载 tar.gz，解压到部署目录，并安装/reload nginx 配置。

```bash
python kaubo-ops deploy 0.5.0 --repo owner/repo --root /var/www/kaubo
```

默认配置：

- 部署根目录：`/var/www/kaubo`
- nginx 配置目标：`/etc/nginx/conf.d/kaubo.conf`
- nginx 配置源：`kaubo-ops/infra/` 下的 nginx 模板

本地开发不要直接运行真实部署命令，除非当前机器就是目标服务器并且有 nginx 权限。

## Benchmark

Benchmark 入口：

```bash
python kaubo-ops bench --lang kaubo python node
python kaubo-ops bench --json --output results/report.json
```

性能基线：

```bash
python kaubo-ops bench --release --save-baseline
```

如果某个 benchmark 样例暂时不兼容当前解释器，应修样例或解释器，不要删除 benchmark 框架。

## 覆盖率

```bash
python kaubo-ops coverage
python kaubo-ops coverage --html
```
