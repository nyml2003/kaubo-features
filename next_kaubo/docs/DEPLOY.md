# Kaubo 部署方案

## 总览

```
                          .version (唯一事实来源)
                          ────────
                         │  0.2.0  │
                         ────────
                         ↑        ↑
                      读取       读取
                       │         │
               scripts/upload.py  scripts/deploy.py
                 (开发机)           (服务器)
                       │               │
                       ▼               ▼
                 GitHub Release ──→ /var/www/kaubo/dist
                       │               │
                       │               ▼
                       │         nginx -s reload
                       │
                  只做一件事：
                  构建 + 打包 + 上传
```

**核心理念：** 开发机和服务器完全解耦，唯一连接点是 GitHub Release。没有 scp，没有 ssh。

---

## 版本管理

文件 `项目根/.version` 一行一个版本号。这是整个部署流程的唯一事实来源。

```
0.2.0
```

- `upload.py` 读它决定发布什么版本
- `deploy.py` 读它决定部署什么版本
- 它被提交到 git，服务器 `git pull` 就能拿到

---

## 开发机：`scripts/upload.py`

**职责：** 构建前端 → 打包 → 上传到 GitHub Release。

**流程：**

```
python3 scripts/upload.py [VERSION] [-y]
         │
         ▼ 读取 .version（或命令行覆盖）
         ▼
  ┌─────────────┐
  │ Step 1      │  pnpm build
  │ 构建        │  在工作区根目录执行
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │ Step 2      │  tar czf kaubo-{version}.tar.gz -C dist .
  │ 打包        │
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │ Step 3      │  gh release create v{version} \
  │ 发布        │    --title "v{version}"        \
  │             │    --notes "描述"               \
  │             │    {tarball}
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │ Step 4      │  删除本地 tar.gz
  │ 清理        │
  └─────────────┘
```

**用法：**

```bash
python3 scripts/upload.py           # 读 .version，交互确认
python3 scripts/upload.py 0.2.0     # 指定版本
python3 scripts/upload.py -y        # 跳过确认，直接发布
```

**前提：**
- 安装了 `pnpm`
- 安装了 `gh` CLI 并已登录（`gh auth login`）
- 仓库已初始化 git

---

## 服务器：`scripts/deploy.py`

**职责：** 清空旧文件 → 下载新版本 → 解压 → 重载 nginx。

**流程：**

```
python3 scripts/deploy.py [VERSION]
         │
         ▼ 读取 .version（或 CLI 参数覆盖）
         ▼
  ┌──────────────────────────────────────────┐
  │ 0. 检查 /var/www/kaubo/.deployed_version │
  │    如果已是最新 → "Already up to date"    │
  │    不是 → 继续                           │
  └────────────────────┬─────────────────────┘
                       ▼
  ┌─────────────┐
  │ Step 1      │  rm -rf /var/www/kaubo/dist/*
  │ 清空        │  保留 dist 目录本身
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │ Step 2      │  urllib GET → 下载 tar.gz
  │ 下载 + 解压 │  tarfile → 解压到 dist/
  │             │  删除 tar.gz
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │ Step 3      │  nginx -t（检查配置语法）
  │ 重载        │  nginx -s reload（重载）
  │             │  写 .deployed_version = {version}
  └─────────────┘
```

**用法：**

```bash
python3 scripts/deploy.py             # 读 .version，部署当前版本
python3 scripts/deploy.py 0.1.0       # 部署/回滚到指定版本
```

**前提：**
- 纯 Python3 stdlib，零 pip 依赖
- 运行在配置了 nginx 的服务器上
- 可访问 GitHub（直连或走代理）

---

## 服务器首次初始化

```bash
# 1. 克隆项目（为了拿到 .version 和 deploy.py）
git clone https://github.com/{owner}/{repo}.git /var/www/kaubo

# 2. 创建 dist 目录
mkdir -p /var/www/kaubo/dist

# 3. 首次部署
cd /var/www/kaubo
python3 scripts/deploy.py
```

---

## 正常发版流程

```bash
# ── 开发机 ──
vim .version                  # 改成 0.2.0
git add .version
git commit -m "release v0.2.0"
git push

python3 scripts/upload.py -y

# ── 服务器 ──
cd /var/www/kaubo
git pull
python3 scripts/deploy.py
# 输出: Downloading v0.2.0... Extracting... nginx reloaded. Done.
```

---

## 回滚

```bash
cd /var/www/kaubo
python3 scripts/deploy.py 0.1.0
# Step 1: 清空当前文件
# Step 2: 下载 v0.1.0 并解压
# Step 3: nginx reload
# Done
```

---

## 相关文件一览

| 文件 | 位置 | 说明 |
|------|------|------|
| `.version` | 项目根 | 当前版本号（提交到 git） |
| `scripts/upload.py` | 项目内 | 构建 + 打包 + 发布到 GitHub Release |
| `scripts/deploy.py` | 项目内 | 清空 + 下载 + 解压 + nginx reload |
| `/var/www/kaubo/dist/` | 服务器 | 部署目标目录 |
| `/var/www/kaubo/.deployed_version` | 服务器 | deploy.py 写入，记录当前已部署版本 |

---

## nginx 配置参考

```nginx
server {
    listen 443 ssl;
    server_name ventusvocatflumen.cn;

    ssl_certificate /etc/nginx/cert/ventusvocatflumen.cn.pem;
    ssl_certificate_key /etc/nginx/cert/ventusvocatflumen.cn.key;

    ssl_session_cache shared:SSL:1m;
    ssl_session_timeout 5m;
    ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE:ECDH:AES:HIGH:!NULL:!aNULL:!MD5:!ADH:!RC4;
    ssl_protocols TLSv1.1 TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers on;

    location / {
        root /var/www/kaubo/dist;
        index index.html;
        try_files $uri $uri/ /index.html;
    }

    location ~ \.wasm$ {
        root /var/www/kaubo/dist;
        add_header Content-Type application/wasm;
        gzip_static on;
    }
}

server {
    listen 80;
    server_name ventusvocatflumen.cn;
    return 301 https://$server_name$request_uri;
}
```
