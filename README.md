# ServerStatus-Plus

轻量级服务器状态监控：Rust 服务端 + Python 3 客户端 + Toyo 原版静态前端。

> 服务端、客户端、安装脚本和文档由 AI 编写；前端基于 [ToyoDAdoubi/ServerStatus-Toyo](https://github.com/ToyoDAdoubi/ServerStatus-Toyo)（MIT）恢复并适配。

## 特性

- Rust + Tokio + Axum 服务端，单文件二进制内嵌原版网页，无需单独部署 Web 目录。
- Python 3 单文件客户端，仅标准库，直接读取 `/proc`，自动重连，自动尝试 IPv4/IPv6。
- 原版前端每 2 秒轮询 `/json/stats.json`，支持展开内存、交换分区和硬盘信息。
- 内存态、无数据库；仓库提供 Linux amd64/arm64 预编译二进制，用户无需编译。

## 要求

- 服务端：Linux `x86_64` / `aarch64`，端口 `35601/tcp`、`8080/tcp`（可配置）。
- 客户端：Linux + Python 3.6+。
- 请在防火墙或云安全组中自行放行端口。

## 安装

```bash
wget --header='Accept: application/vnd.github.raw' -O status.sh \
  'https://api.github.com/repos/ztoofar/ServerStatus-Plus/contents/status.sh?ref=main'
chmod +x status.sh

bash status.sh s   # 服务端菜单
bash status.sh c   # 客户端菜单
```

如果 GitHub API 不可用，可改用 jsDelivr（可能有最多 12 小时缓存）：

```bash
wget -N --no-check-certificate https://cdn.jsdelivr.net/gh/ztoofar/ServerStatus-Plus@main/status.sh
```

菜单：`0` 升级脚本、`1` 安装、`2` 更新、`3` 卸载、`4/5/6` 启动/停止/重启、`7` 配置、`8` 查看信息、`9` 查看日志、`10` 切换菜单、`q` 退出。

代理安装：

```bash
export http_proxy=http://127.0.0.1:7890
export https_proxy=http://127.0.0.1:7890
bash status.sh s
# 或：bash status.sh s --proxy http://127.0.0.1:7890
```

`wget` 不支持 SOCKS5 代理。服务端和客户端可部署在同一台机器，客户端 `SERVER` 填 `127.0.0.1` 即可。

## 目录与服务

| 路径 | 说明 |
| --- | --- |
| `/usr/local/ServerStatus/server/sergate` | 服务端二进制（内嵌网页） |
| `/usr/local/ServerStatus/server/config.json` | 服务端配置 |
| `/usr/local/ServerStatus/client/sergate-client.py` | Python 客户端脚本（配置在文件顶部） |

```bash
service status-server start|stop|restart|status
service status-client start|stop|restart|status

tail -f /tmp/serverstatus_server.log
tail -f /tmp/serverstatus_client.log
```

服务通过 `setsid` 独立会话启动，退出安装脚本或断开 SSH 不会停止服务。

## 配置

### 服务端

配置文件为 `/usr/local/ServerStatus/server/config.json`，可直接使用原版 Toyo 的精简格式：

```json
{
  "servers": [
    {
      "username": "s01",
      "password": "password",
      "name": "Server 01",
      "type": "KVM",
      "host": "",
      "location": "Hong Kong"
    }
  ]
}
```

可选字段：`listen`（默认 `:35601`）、`http_listen`（默认 `:8080`）、`site.title`、`site.announcement`；节点加 `"disabled": true` 可隐藏。缺失的 `id` 和 `name` 默认使用 `username`。完整示例见 [config.example.json](config.example.json)。

改完后重启：

```bash
service status-server restart
```

### 客户端

客户端只有一个 `/usr/local/ServerStatus/client/sergate-client.py`，配置在文件顶部：

```python
SERVER = "127.0.0.1"
PORT = 35601
USERNAME = "s01"
PASSWORD = "password"
```

可用 `bash status.sh c` 菜单 `7` 修改，或直接编辑文件；改完重启 `status-client`。

- 自动尝试 IPv4/IPv6，在线状态由服务端根据实际连接判断。
- 上报间隔固定为 1 秒。
- 负载：已建立 TCP 连接的唯一远端 IPv4/IPv6 地址数（排除 `127.0.0.0/8`、`::1` 等本地地址）。

## 构建与测试

需要 Rust 1.85+；前端是静态文件，无需 Node.js/npm；客户端无需编译。

```bash
cargo build --release
cargo zigbuild --release --target x86_64-unknown-linux-musl
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

`bin/sergate-linux-{amd64,arm64}` 是安装脚本使用的预编译产物；修改服务端或 `web/` 后必须重建并提交。

```bash
cargo test
python tests/test_client.py
python tests/client_smoke_test.py   # 仅 Linux

# 另开终端启动测试服务端
target/release/sergate -c tests/fixtures/smoke-config.json
python -I tests/smoke_test.py
```

## 维护

修改 `status.sh` 后必须递增文件顶部的 `sh_ver`。

## License

MIT。前端来自 [ToyoDAdoubi/ServerStatus-Toyo](https://github.com/ToyoDAdoubi/ServerStatus-Toyo)（MIT）；Bootstrap 3（Apache-2.0）、jQuery 1.10.2（MIT）。
