# ServerStatus-Plus

轻量、实时的服务器监控面板：

- **Rust 服务端**：Tokio + Axum，单文件静态二进制，内嵌 Web 前端
- **Rust 客户端**：单文件静态二进制，读取 `/proc` 采集指标
- **Vue 3 前端**：Vite 构建，SSE 实时刷新，深色/浅色主题
- **IPv4 / IPv6 双栈**：服务端双栈监听，客户端自动或强制选择协议族
- **无历史存储**：数据只在内存中，重启清空，不写数据库、不写 `stats.json`
- **预编译二进制**：`bin/` 已包含 Linux amd64 / arm64，安装时直接下载，无需编译

> 仓库必须为 **Public**，否则服务器无法下载脚本和二进制。

## 一键安装

```bash
wget -N --no-check-certificate https://raw.githubusercontent.com/ztoofar/ServerStatus-Plus/main/status.sh && chmod +x status.sh
```

```bash
bash status.sh s   # 服务端菜单
bash status.sh c   # 客户端菜单
```

菜单与原版一致：`1` 安装、`2` 更新、`3` 卸载、`4/5/6` 启动/停止/重启、`7` 配置、`8` 查看信息、`9` 查看日志、`10` 切换菜单。

服务端和客户端可装在同一台机器：先装服务端，再切到客户端菜单安装，客户端地址填 `127.0.0.1`。卸载时会保留另一端。

### 通过代理安装

脚本内所有 `wget`（二进制、服务脚本、jq、脚本自更新）都支持 HTTP/HTTPS 代理：

```bash
# 先设置环境变量，再下载 status.sh；脚本会自动继承
export http_proxy=http://127.0.0.1:7890
export https_proxy=http://127.0.0.1:7890
wget -N --no-check-certificate https://raw.githubusercontent.com/ztoofar/ServerStatus-Plus/main/status.sh && chmod +x status.sh
bash status.sh s
```

也可以给脚本传参：

```bash
bash status.sh s --proxy http://127.0.0.1:7890
```

`wget` 不支持 `socks5://`，请填写代理软件（如 Clash）的 HTTP/HTTPS 端口。

## 目录与端口

```text
/usr/local/ServerStatus/
├── server/sergate         # 服务端（内嵌 Web）
├── server/config.json
├── client/sergate-client  # 客户端
└── client/config.json
```

- `35601/tcp`：客户端上报
- `8080/tcp`：Web 面板（安装时可修改）

```bash
service status-server start|stop|restart|status
service status-client start|stop|restart|status
tail -f /tmp/serverstatus_server.log
tail -f /tmp/serverstatus_client.log
```

脚本不再安装 Caddy。需要 HTTPS 时，用 Nginx / Caddy v2 反向代理到面板端口即可。

## 配置

服务端 `server/config.json`：

```json
{
  "listen": ":35601",
  "http_listen": ":8080",
  "site": { "title": "ServerStatus-Plus", "announcement": "" },
  "servers": [
    {
      "id": "username01",
      "username": "username01",
      "password": "password",
      "name": "Server 01",
      "type": "KVM",
      "host": "",
      "location": "Hong Kong",
      "group": "",
      "tags": [],
      "disabled": false
    }
  ]
}
```

客户端 `client/config.json`：

```json
{
  "server": "127.0.0.1",
  "port": 35601,
  "username": "username01",
  "password": "password",
  "family": 0,
  "interval": 1,
  "online_check_v4": "www.aliyun.com:443",
  "online_check_v6": "www.aliyun.com:443",
  "interfaces": []
}
```

- `family`：`0` 自动，`4` 仅 IPv4，`6` 仅 IPv6。
- `interfaces` 为空时自动识别物理网卡。
- 服务端 `listen`/`http_listen` 使用 `:端口` 时默认双栈监听。
- 修改配置后重启对应服务。

## 网页数据

页面保持原来的表格数据，没有历史曲线、管理后台或告警：

状态（IPv4/IPv6）、节点名、虚拟化、位置、在线时间、负载、网络 ↓/↑、流量 ↓/↑、CPU、内存、硬盘；点击行展开内存、交换分区、硬盘和自定义信息。

兼容旧接口 `/json/stats.json`，但内容由内存实时生成，不落盘。

## 从源码构建

需要 Rust 1.99+、Node.js 22+、npm；Linux 静态交叉编译使用 `cargo-zigbuild` + Zig。

```bash
cd web && npm install && npm run build && cd ..
cargo build --release
cargo zigbuild --release --target x86_64-unknown-linux-musl
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

产物为 `target/<target>/release/sergate` 和 `sergate-client`。

## License

MIT，详见 [LICENSE](LICENSE)。
