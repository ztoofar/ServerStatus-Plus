# ServerStatus-Plus

> **AI 编写声明**
>
> 本项目的 Rust 服务端、Python 客户端、安装脚本及文档由 AI 编写；前端页面基于 ToyoDAdoubi/ServerStatus-Toyo 的原始静态页面（MIT）恢复并做少量适配。

ServerStatus-Plus 是一套轻量级、实时的服务器状态监控系统。服务端使用 Rust 构建并内嵌网页，客户端使用 Python 3 直接读取 `/proc` 采集指标，前端为 Toyo 原版静态页面（Bootstrap 3 + jQuery），支持 IPv4/IPv6 双栈与内存态实时展示。

## 主要特性

- **服务端**：Rust + Tokio + Axum，编译为单文件静态二进制，原版网页资源直接内嵌，无需单独部署 `web` 目录。
- **客户端**：Python 3 单文件实现，仅使用标准库，直接读取 `/proc` 与 `/sys`，无 `netstat`/`df` 子进程，自动重连。
- **前端**：Toyo 原版静态页面，每 2 秒轮询 `/json/stats.json`，支持点击行展开内存、交换分区、硬盘与自定义信息。
- **网络**：服务端 TCP/HTTP 同时支持 IPv4/IPv6（IPv6 不可用时自动回退 IPv4）；客户端 `family` 可选 `0`（自动）、`4`（仅 IPv4）、`6`（仅 IPv6）。
- **配置**：服务端可直接使用原版 Toyo 的 `config.json`（只含 `servers` 数组），也兼容包含 `listen`/`http_listen`/`site` 的扩展格式。
- **数据**：仅使用内存态，不持久化历史数据，不依赖数据库。
- **部署**：仓库提供 Linux amd64/arm64 预编译服务端二进制，安装脚本直接下载，用户服务器无需 Rust/Node 编译环境。

## 环境要求

- 服务端：Linux（推荐 Debian 12/13），`x86_64` 或 `aarch64`。
- 客户端：Linux，Python 3.6+（CentOS 7 / Debian 10+ 自带或可安装）。
- 端口：`35601/tcp`（客户端上报）、`8080/tcp`（Web 面板，可配置）。
- 请在防火墙或云安全组中自行放行上述端口（安装脚本不会修改防火墙）。

## 安装

下载并执行安装脚本（GitHub API 始终返回最新内容）：

```bash
wget --header='Accept: application/vnd.github.raw' -O status.sh \
  'https://api.github.com/repos/ztoofar/ServerStatus-Plus/contents/status.sh?ref=main' && chmod +x status.sh
```

备用下载地址：

```bash
# jsDelivr CDN（可能缓存最多 12 小时）
wget -N --no-check-certificate https://cdn.jsdelivr.net/gh/ztoofar/ServerStatus-Plus@main/status.sh && chmod +x status.sh

# GitHub Raw（部分网络可能返回 404 或连接重置）
wget -N --no-check-certificate https://raw.githubusercontent.com/ztoofar/ServerStatus-Plus/main/status.sh && chmod +x status.sh
```

脚本内部下载客户端、服务端二进制和服务脚本时按 GitHub Raw → GitHub API → jsDelivr 顺序自动回退。

根据需要进入服务端或客户端菜单：

```bash
bash status.sh s   # 服务端菜单
bash status.sh c   # 客户端菜单
```

菜单项：`0` 升级脚本、`1` 安装、`2` 更新、`3` 卸载、`4/5/6` 启动/停止/重启、`7` 配置、`8` 查看信息、`9` 查看日志、`10` 切换菜单、`q` 退出。

服务端与客户端可部署于同一台服务器。客户端配置中的服务端地址填写 `127.0.0.1` 即可；卸载时仅移除当前组件，保留另一端。

### 代理安装

如服务器需要通过代理访问外网，可先设置环境变量：

```bash
export http_proxy=http://127.0.0.1:7890
export https_proxy=http://127.0.0.1:7890
wget --header='Accept: application/vnd.github.raw' -O status.sh \
  'https://api.github.com/repos/ztoofar/ServerStatus-Plus/contents/status.sh?ref=main' && chmod +x status.sh
bash status.sh s
```

也可在运行脚本时指定代理：

```bash
bash status.sh s --proxy http://127.0.0.1:7890
```

`wget` 不支持 `socks5://` 代理，请使用 HTTP/HTTPS 代理端口。

## 目录与服务

| 路径 | 说明 |
| --- | --- |
| `/usr/local/ServerStatus/server/sergate` | 服务端二进制（内嵌网页） |
| `/usr/local/ServerStatus/server/config.json` | 服务端配置 |
| `/usr/local/ServerStatus/client/sergate-client.py` | Python 客户端脚本 |
| `/usr/local/ServerStatus/client/config.json` | 客户端配置 |

服务管理：

```bash
service status-server start|stop|restart|status
service status-client start|stop|restart|status
```

服务通过 `setsid` 以独立会话启动，退出安装脚本或断开 SSH 不会停止服务。

日志：

```bash
tail -f /tmp/serverstatus_server.log
tail -f /tmp/serverstatus_client.log
```

## 配置说明

### 服务端

配置文件为 `server/config.json`。每个节点的 `username` 必须唯一，`password` 需与客户端配置一致。

可以直接使用原版 Toyo 的精简格式：

```json
{
  "servers": [
    {
      "username": "s01",
      "password": "password",
      "name": "Server 01",
      "type": "KVM",
      "host": "",
      "location": "Hong Kong",
      "disabled": false
    }
  ]
}
```

缺失的 `listen`、`http_listen`、`site`、`id`、`name`、`group`、`tags` 会自动使用默认值：`listen=":35601"`、`http_listen=":8080"`、标题为 `ServerStatus-Plus`。也可以使用 [config.example.json](config.example.json) 的扩展格式。

安装脚本添加的新节点会插入 `servers` 数组最前面，因此显示在表格最上方。

### 客户端

配置文件为 `client/config.json`：

```json
{
  "server": "127.0.0.1",
  "port": 35601,
  "username": "s01",
  "password": "password",
  "family": 0,
  "interval": 1,
  "load_mode": "loadavg",
  "online_check_v4": "www.aliyun.com:443",
  "online_check_v6": "www.aliyun.com:443",
  "interfaces": []
}
```

- `family`：`0` 自动、`4` 仅 IPv4、`6` 仅 IPv6。
- `interval`：上报间隔秒数，默认 `1`，必须小于服务端 30 秒空闲超时（建议 `<= 25`）。
- `load_mode`：`loadavg`（默认，系统 1 分钟平均负载，与 `uptime` 一致）或 `connections`（原版 Toyo 语义，已建立 TCP 的唯一远端 IP 数）。
- `interfaces`：留空自动选择物理网卡；也可填写网卡名列表。
- 修改配置后需重启客户端服务。

## 网页数据

页面展示状态（IPv4 开启/关闭）、节点名、虚拟化、位置、在线时间、负载、网络、流量、CPU、内存与硬盘信息；点击行可展开内存、交换分区、硬盘和自定义信息。

页面每 2 秒请求 `/json/stats.json`，该接口由内存实时生成，不写入磁盘；`/api/status`、`/api/events` 等现代接口保留用于脚本或二次开发，但原版页面不使用 SSE。

## 构建

构建服务端需要 Rust 1.85+；前端为静态文件，无需 Node.js/npm。客户端为 Python 脚本，无需编译。

```bash
cargo build --release
cargo zigbuild --release --target x86_64-unknown-linux-musl
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

构建产物位于 `target/<target>/release/sergate`。安装脚本下载的预编译二进制位于 `bin/sergate-linux-{amd64,arm64}`；修改 `web/` 或服务端代码后必须重新构建并提交这两个文件。

测试：

```bash
cargo test
python tests/test_client.py
python tests/client_smoke_test.py   # 仅 Linux
# 另开终端启动测试服务端：
target/release/sergate -c tests/fixtures/smoke-config.json
# 然后运行端到端测试：
python -I tests/smoke_test.py
```

## License

MIT，详见 [LICENSE](LICENSE)。

前端资源来自 [ToyoDAdoubi/ServerStatus-Toyo](https://github.com/ToyoDAdoubi/ServerStatus-Toyo)（MIT）；Bootstrap 3 为 Apache-2.0，jQuery 1.10.2 为 MIT。

## 致谢

感谢 Toyo 开源的 ServerStatus-Toyo 项目，本项目的界面与早期设计受其启发。
