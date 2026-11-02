# ServerStatus-Plus

> **AI 编写声明**
>
> 本项目的服务端、客户端、前端、安装脚本及文档均由 AI 编写。

ServerStatus-Plus 是一套轻量级、实时的服务器状态监控系统。系统采用 Rust 构建服务端与客户端，前端基于 Vue 3，支持 IPv4/IPv6 双栈与内存态实时展示。

## 主要特性

- **服务端**：基于 Rust、Tokio 与 Axum 实现，编译为单文件静态二进制，并内嵌 Web 资源。
- **客户端**：基于 Rust 实现，编译为单文件静态二进制，通过读取 `/proc` 采集系统指标。
- **前端**：基于 Vue 3 与 Vite 构建，通过 SSE 接收实时数据，支持深色/浅色主题。
- **网络**：服务端支持 IPv4/IPv6 双栈监听；客户端可自动选择或指定协议族。
- **数据**：仅使用内存态，不持久化历史数据，不依赖数据库。
- **部署**：仓库提供 Linux amd64/arm64 预编译二进制，安装脚本直接下载，无需在服务器编译。

## 环境要求

- 操作系统：Linux（推荐 Debian 12/13）
- CPU 架构：`x86_64` 或 `aarch64`
- 端口：`35601/tcp`（客户端上报）、`8080/tcp`（Web 面板，可配置）

## 安装

下载并执行安装脚本：

```bash
wget -N --no-check-certificate https://raw.githubusercontent.com/ztoofar/ServerStatus-Plus/main/status.sh && chmod +x status.sh
```

根据需要进入服务端或客户端菜单：

```bash
bash status.sh s   # 服务端菜单
bash status.sh c   # 客户端菜单
```

菜单项：`1` 安装、`2` 更新、`3` 卸载、`4/5/6` 启动/停止/重启、`7` 配置、`8` 查看信息、`9` 查看日志、`10` 切换菜单。

服务端与客户端可部署于同一台服务器。客户端配置中的服务端地址填写 `127.0.0.1` 即可；卸载时仅移除当前组件，保留另一端。

### 代理安装

如服务器需要通过代理访问外网，可先设置环境变量：

```bash
export http_proxy=http://127.0.0.1:7890
export https_proxy=http://127.0.0.1:7890
wget -N --no-check-certificate https://raw.githubusercontent.com/ztoofar/ServerStatus-Plus/main/status.sh && chmod +x status.sh
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
| `/usr/local/ServerStatus/server/sergate` | 服务端二进制 |
| `/usr/local/ServerStatus/server/config.json` | 服务端配置 |
| `/usr/local/ServerStatus/client/sergate-client` | 客户端二进制 |
| `/usr/local/ServerStatus/client/config.json` | 客户端配置 |

服务管理：

```bash
service status-server start|stop|restart|status
service status-client start|stop|restart|status
```

日志：

```bash
tail -f /tmp/serverstatus_server.log
tail -f /tmp/serverstatus_client.log
```

## 配置说明

服务端配置文件为 `server/config.json`，包含 `listen`、`http_listen`、`site` 与 `servers`。每个节点的 `username` 必须唯一，`password` 需与客户端配置一致。完整示例见 [config.example.json](config.example.json)。

客户端配置文件为 `client/config.json`。其中 `family` 可选 `0`（自动）、`4`（仅 IPv4）、`6`（仅 IPv6）。修改配置后需重启对应服务。

## 网页数据

页面展示状态（IPv4/IPv6）、节点名、虚拟化、位置、在线时间、负载、网络、流量、CPU、内存与硬盘信息；点击行可展开内存、交换分区、硬盘及自定义信息。页面不包含历史曲线、管理后台或告警功能。

兼容接口 `/json/stats.json` 由内存实时生成，不写入磁盘。

## 构建

构建需要 Rust 1.99+、Node.js 22+ 与 npm。Linux 静态交叉编译使用 `cargo-zigbuild` 与 Zig。

```bash
cd web && npm install && npm run build && cd ..
cargo build --release
cargo zigbuild --release --target x86_64-unknown-linux-musl
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

构建产物位于 `target/<target>/release/`，包含 `sergate` 与 `sergate-client`。

## License

MIT，详见 [LICENSE](LICENSE)。

## 致谢

感谢 Toyo 开源的 ServerStatus-Toyo 项目，本项目的早期设计及一键脚本结构受其启发。
