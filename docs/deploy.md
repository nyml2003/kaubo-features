# 发布和部署

目标读者：维护 Web Playground 发布、部署和运维入口的开发者。

## 当前状态

仓库没有 GitHub Actions workflow。当前本地 CI/CD 入口统一收口到根目录 `Makefile.toml`：

- release publish：`cargo make ops-release ...`
- server deploy：`cargo make ops-deploy ...`
- coverage report：`cargo make ops-coverage` / `cargo make ops-coverage-html`
- benchmark：`cargo make ops-bench` / `cargo make ops-bench-check`

这些 cargo-make 任务仍调用 `next_kaubo/ops/` 下的 Python 脚本。脚本可以后续接入 GitHub Actions 或其他 CI/CD 系统。

## 发布 Release

发布脚本会读取或更新 `.version`，构建 Web app，把 `dist/` 打成 `kaubo-vX.Y.Z.tar.gz`，然后通过 `gh release create` 上传到 GitHub Release。

```bash
cargo make ops-release --bump patch
cargo make ops-release 0.5.0 -y
```

前提：

- 已安装 `cargo-make`。
- 已安装 `pnpm` 和 GitHub CLI `gh`。
- 已执行 `gh auth login`。

## 部署到服务器

部署脚本从 GitHub Release 下载 tar.gz，解压到部署目录，并安装/reload nginx 配置。

```bash
cargo make ops-deploy 0.5.0 --repo owner/repo --root /var/www/kaubo
```

默认配置：

- 部署根目录：`/var/www/kaubo`
- nginx 配置目标：`/etc/nginx/conf.d/kaubo.conf`
- nginx 配置源：`ops/deploy/nginx/kaubo.conf`

本地开发不要直接运行真实部署命令，除非当前机器就是目标服务器并且有 nginx 权限。

## Benchmark

Benchmark 入口：

```bash
cargo make ops-bench
cd next_kaubo
python3 ops/benchmark/runner.py bench --lang kaubo python rust
python3 ops/benchmark/runner.py bench --json --output results/report.json
```

性能基线：

```bash
cd next_kaubo
python3 ops/benchmark/runner.py bench --release --save-baseline
cargo make ops-bench-check
```

如果某个 benchmark 样例暂时不兼容当前解释器，应修样例或解释器，不要删除 benchmark 框架。
