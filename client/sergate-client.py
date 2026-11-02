#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""ServerStatus-Plus monitoring client for Linux.

Reads metrics directly from /proc and /sys, authenticates against the Rust
sergate server over TCP, and reports one ``update <json>`` line per interval.
Only the Python standard library is required (Python 3.6+).
"""

import argparse
import collections
import ipaddress
import json
import os
import signal
import socket
import sys
import time

# 客户端配置：安装脚本会修改下面四项。
SERVER = "127.0.0.1"
PORT = 35601
USERNAME = "s01"
PASSWORD = "password"

UPDATE_INTERVAL = 1.0

CONNECT_TIMEOUT = 10.0
AUTH_TIMEOUT = 10.0
SEND_TIMEOUT = 15.0
RECONNECT_MIN = 3.0
RECONNECT_MAX = 30.0
AUTH_LIMIT = 4096
REFRESH_INTERVAL = 60.0

EXCLUDED_PREFIXES = (
    "lo", "docker", "br-", "veth", "virbr", "tun", "tap", "wg", "tailscale",
    "zt", "cni", "flannel", "podman", "lxc", "vmnet", "vnet", "macvlan",
    "ipvlan", "ifb", "dummy",
)

EXCLUDED_FS = frozenset((
    "proc", "sysfs", "devtmpfs", "tmpfs", "devpts", "cgroup", "cgroup2",
    "securityfs", "debugfs", "tracefs", "configfs", "pstore", "mqueue",
    "hugetlbfs", "rpc_pipefs", "autofs", "overlay", "squashfs", "fusectl",
    "bpf", "nfs", "nfs4", "cifs", "smb3", "sshfs", "9p", "ceph", "glusterfs",
))

STOP = False


class ConfigError(Exception):
    pass


class AuthError(Exception):
    pass


class ConnectError(Exception):
    pass


class ClientConfig(object):
    __slots__ = ("server", "port", "username", "password")

    def __init__(self, server, port, username, password):
        self.server = server
        self.port = port
        self.username = username
        self.password = password


class Logger(object):
    def __init__(self, verbose=False):
        self.verbose = verbose
        self._last_error = None
        self._last_error_at = 0.0

    def info(self, message):
        self._write(message)

    def debug(self, message):
        if self.verbose:
            self._write(message)

    def error(self, message):
        now = time.monotonic()
        if message != self._last_error or now - self._last_error_at >= 30.0:
            self._write("错误: " + message)
            self._last_error = message
            self._last_error_at = now

    @staticmethod
    def _write(message):
        sys.stdout.write("%s %s\n" % (time.strftime("%Y-%m-%d %H:%M:%S"), message))
        sys.stdout.flush()


def make_config(server=SERVER, port=PORT, username=USERNAME, password=PASSWORD):
    server = str(server).strip()
    username = str(username).strip()
    password = str(password)
    if not server:
        raise ConfigError("SERVER 不能为空")
    if not username:
        raise ConfigError("USERNAME 不能为空")
    if not password:
        raise ConfigError("PASSWORD 不能为空")
    if any(ch in username + password for ch in ("\r", "\n")):
        raise ConfigError("USERNAME/PASSWORD 不能包含换行符")

    try:
        port = int(port)
    except (TypeError, ValueError):
        raise ConfigError("PORT 必须是整数")
    if not 1 <= port <= 65535:
        raise ConfigError("PORT 必须在 1-65535 之间")

    return ClientConfig(server, port, username, password)


def build_config():
    return make_config(SERVER, PORT, USERNAME, PASSWORD)


def install_signal_handlers():
    def handler(_signum, _frame):
        global STOP
        STOP = True

    signal.signal(signal.SIGINT, handler)
    signal.signal(signal.SIGTERM, handler)


def sleep_with_stop(seconds):
    end = time.monotonic() + seconds
    while not STOP:
        remaining = end - time.monotonic()
        if remaining <= 0:
            break
        time.sleep(min(remaining, 0.5))


def read_uptime():
    try:
        with open("/proc/uptime", "r") as handle:
            return int(float(handle.readline().split()[0]))
    except (OSError, ValueError, IndexError):
        return 0


def parse_meminfo(lines):
    values = {}
    for line in lines:
        parts = line.split()
        if len(parts) < 2:
            continue
        try:
            values[parts[0].rstrip(":")] = int(parts[1])
        except ValueError:
            continue
    return values


def memory_values(values):
    total = max(0, int(values.get("MemTotal", 0)))
    fallback_available = (
        int(values.get("MemFree", 0))
        + int(values.get("Buffers", 0))
        + int(values.get("Cached", 0))
        + int(values.get("SReclaimable", 0))
        - int(values.get("Shmem", 0))
    )
    available = int(values.get("MemAvailable", fallback_available))
    available = max(0, min(available, total))
    swap_total = max(0, int(values.get("SwapTotal", 0)))
    swap_free = max(0, int(values.get("SwapFree", 0)))
    return total, max(0, total - available), swap_total, max(0, swap_total - swap_free)


def read_memory():
    try:
        with open("/proc/meminfo", "r") as handle:
            values = parse_meminfo(handle)
    except OSError:
        return 0, 0, 0, 0
    return memory_values(values)


def read_cpu_sample():
    try:
        with open("/proc/stat", "r") as handle:
            parts = handle.readline().split()
    except OSError:
        return None
    if len(parts) < 9 or parts[0] != "cpu":
        return None
    try:
        values = [int(value) for value in parts[1:9]]
    except ValueError:
        return None
    return sum(values), values[3] + values[4]


class CpuSampler(object):
    def __init__(self):
        self.previous = None

    def sample(self):
        current = read_cpu_sample()
        if current is None:
            return 0
        if self.previous is None:
            self.previous = current
            return 0
        total, idle = current
        old_total, old_idle = self.previous
        self.previous = current
        total_delta = total - old_total
        idle_delta = idle - old_idle
        if total_delta <= 0:
            return 0
        usage = 100.0 * (total_delta - idle_delta) / total_delta
        if usage < 0.0:
            usage = 0.0
        elif usage > 100.0:
            usage = 100.0
        return int(usage + 0.5)


def decode_mount(path):
    if "\\" not in path:
        return path
    output = []
    index = 0
    while index < len(path):
        if (
            path[index] == "\\"
            and index + 3 < len(path)
            and all(ch in "01234567" for ch in path[index + 1:index + 4])
        ):
            output.append(chr(int(path[index + 1:index + 4], 8)))
            index += 4
        else:
            output.append(path[index])
            index += 1
    return "".join(output)


def find_mounts():
    mounts = []
    devices = set()
    try:
        with open("/proc/self/mounts", "r", encoding="utf-8", errors="replace") as handle:
            for line in handle:
                fields = line.split()
                if len(fields) < 3 or fields[2] in EXCLUDED_FS:
                    continue
                mount = decode_mount(fields[1])
                try:
                    stat = os.stat(mount)
                except OSError:
                    continue
                if stat.st_dev in devices:
                    continue
                devices.add(stat.st_dev)
                mounts.append(mount)
    except OSError:
        return []
    return mounts


class DiskSampler(object):
    def __init__(self):
        self.mounts = []
        self.next_refresh = 0.0

    def sample(self):
        now = time.monotonic()
        if now >= self.next_refresh:
            self.mounts = find_mounts()
            self.next_refresh = now + REFRESH_INTERVAL

        total = 0
        used = 0
        for mount in self.mounts or ("/",):
            try:
                stat = os.statvfs(mount)
            except OSError:
                continue
            block_size = stat.f_frsize or stat.f_bsize
            total += stat.f_blocks * block_size
            used += (stat.f_blocks - stat.f_bfree) * block_size
        return total // 1048576, used // 1048576


def scan_interfaces():
    try:
        names = os.listdir("/sys/class/net")
    except OSError:
        return []

    result = []
    for name in names:
        if any(name.startswith(prefix) for prefix in EXCLUDED_PREFIXES):
            continue
        base = os.path.join("/sys/class/net", name)
        try:
            with open(os.path.join(base, "type"), "r") as handle:
                if handle.read().strip() != "1":
                    continue
            if not os.path.exists(os.path.join(base, "device")):
                continue
            with open(os.path.join(base, "ifindex"), "r") as handle:
                ifindex = int(handle.read().strip())
            with open(os.path.join(base, "iflink"), "r") as handle:
                iflink = int(handle.read().strip())
        except (OSError, ValueError):
            continue
        if ifindex == iflink:
            result.append(name)
    return result


def read_network_counters(names):
    try:
        with open("/proc/net/dev", "r") as handle:
            next(handle, None)
            next(handle, None)
            selected = set(names) if names else None
            rx_total = 0
            tx_total = 0
            for line in handle:
                if ":" not in line:
                    continue
                name, values = line.split(":", 1)
                name = name.strip()
                if selected is None:
                    if name == "lo" or name.startswith("tun"):
                        continue
                elif name not in selected:
                    continue
                fields = values.split()
                if len(fields) < 16:
                    continue
                try:
                    rx_total += int(fields[0])
                    tx_total += int(fields[8])
                except ValueError:
                    continue
            return rx_total, tx_total
    except OSError:
        return 0, 0


class NetworkSampler(object):
    def __init__(self):
        self.auto_names = []
        self.next_refresh = 0.0
        self.previous = None
        self.rx_history = collections.deque(maxlen=5)
        self.tx_history = collections.deque(maxlen=5)

    def _interfaces(self, now):
        if not self.auto_names or now >= self.next_refresh:
            self.auto_names = scan_interfaces()
            self.next_refresh = now + REFRESH_INTERVAL
        return self.auto_names

    def sample(self):
        now = time.monotonic()
        rx_total, tx_total = read_network_counters(self._interfaces(now))
        rx_speed = 0
        tx_speed = 0

        if self.previous is not None:
            previous_time, previous_rx, previous_tx = self.previous
            elapsed = now - previous_time
            if elapsed > 0.0:
                if rx_total >= previous_rx:
                    rx_speed = int((rx_total - previous_rx) / elapsed)
                if tx_total >= previous_tx:
                    tx_speed = int((tx_total - previous_tx) / elapsed)
                self.rx_history.append(rx_speed)
                self.tx_history.append(tx_speed)
                if self.rx_history:
                    rx_speed = sum(self.rx_history) // len(self.rx_history)
                if self.tx_history:
                    tx_speed = sum(self.tx_history) // len(self.tx_history)

        self.previous = (now, rx_total, tx_total)
        return rx_speed, tx_speed, rx_total, tx_total


def decode_tcp_address(raw, is_ipv6):
    try:
        data = bytes.fromhex(raw)
    except ValueError:
        return None

    if is_ipv6:
        if len(data) != 16:
            return None
        data = b"".join(data[index:index + 4][::-1] for index in range(0, 16, 4))
        try:
            address = ipaddress.IPv6Address(data)
        except ValueError:
            return None
        if address.ipv4_mapped is not None:
            return str(address.ipv4_mapped)
        return str(address)

    if len(data) != 4:
        return None
    return socket.inet_ntoa(data[::-1])


def is_local_address(address):
    return address in ("0.0.0.0", "::") or address == "::1" or address.startswith("127.")


def parse_tcp_table(lines, is_ipv6, addresses):
    first = True
    for line in lines:
        if first:
            first = False
            continue
        fields = line.split()
        if len(fields) < 4 or fields[3] != "01":
            continue
        remote = fields[2]
        if ":" not in remote:
            continue
        address = decode_tcp_address(remote.rsplit(":", 1)[0], is_ipv6)
        if address is not None and not is_local_address(address):
            addresses.add(address)


def read_established_load():
    # 负载：连接到本机的唯一远端 IPv4/IPv6 地址数（排除回环）。
    addresses = set()
    for path, is_ipv6 in (("/proc/net/tcp", False), ("/proc/net/tcp6", True)):
        try:
            with open(path, "r") as handle:
                parse_tcp_table(handle, is_ipv6, addresses)
        except OSError:
            continue
    return len(addresses)


def connect(config):
    try:
        infos = socket.getaddrinfo(
            config.server, config.port, socket.AF_UNSPEC, socket.SOCK_STREAM
        )
    except OSError as exc:
        raise ConnectError("解析地址失败: %s" % exc)

    last_error = None
    seen = set()
    for af_value, socktype, proto, _canonname, sockaddr in infos:
        key = (af_value, sockaddr)
        if key in seen:
            continue
        seen.add(key)
        sock = socket.socket(af_value, socktype, proto)
        try:
            sock.settimeout(CONNECT_TIMEOUT)
            try:
                sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            except OSError:
                pass
            sock.connect(sockaddr)
            return sock
        except OSError as exc:
            last_error = exc
            try:
                sock.close()
            except OSError:
                pass

    if last_error is None:
        raise ConnectError("没有可用地址")
    raise ConnectError("无法连接 %s:%s: %s" % (config.server, config.port, last_error))


def recv_until(sock, markers, limit=AUTH_LIMIT, timeout=AUTH_TIMEOUT):
    sock.settimeout(timeout)
    data = bytearray()
    while len(data) < limit:
        try:
            chunk = sock.recv(min(1024, limit - len(data)))
        except socket.timeout:
            break
        if not chunk:
            break
        data.extend(chunk)
        text = data.decode("utf-8", "replace")
        if any(marker in text for marker in markers):
            return text
    return bytes(data).decode("utf-8", "replace")


def authenticate(sock, config):
    greeting = recv_until(sock, ("Authentication required",))
    if "Authentication required" not in greeting:
        raise AuthError("服务端未请求认证: %s" % greeting.strip()[:200])

    sock.sendall((config.username + ":" + config.password + "\n").encode("utf-8"))
    response = recv_until(
        sock,
        ("Authentication failed", "Only one connection", "IPv4", "IPv6"),
    )
    if "Authentication failed" in response or "Only one connection" in response:
        raise AuthError(response.strip() or "认证失败")
    if "Authentication successful" not in response:
        raise AuthError("认证未完成: %s" % response.strip()[:200])


def build_payload(cpu_sampler, network_sampler, disk_sampler):
    memory_total, memory_used, swap_total, swap_used = read_memory()
    hdd_total, hdd_used = disk_sampler.sample()
    rx_speed, tx_speed, rx_total, tx_total = network_sampler.sample()
    return {
        "uptime": read_uptime(),
        "load": float(read_established_load()),
        "memory_total": memory_total,
        "memory_used": memory_used,
        "swap_total": swap_total,
        "swap_used": swap_used,
        "hdd_total": hdd_total,
        "hdd_used": hdd_used,
        "cpu": cpu_sampler.sample(),
        "network_rx": rx_speed,
        "network_tx": tx_speed,
        "network_in": rx_total,
        "network_out": tx_total,
    }


def run(config, once, logger):
    cpu_sampler = CpuSampler()
    network_sampler = NetworkSampler()
    disk_sampler = DiskSampler()
    backoff = RECONNECT_MIN

    while not STOP:
        sock = None
        try:
            sock = connect(config)
            authenticate(sock, config)
            sock.settimeout(SEND_TIMEOUT)
            logger.info("已连接 %s:%s" % (config.server, config.port))
            backoff = RECONNECT_MIN
            deadline = time.monotonic()

            while not STOP:
                payload = build_payload(cpu_sampler, network_sampler, disk_sampler)
                line = "update " + json.dumps(payload, separators=(",", ":")) + "\n"
                sock.sendall(line.encode("utf-8"))
                logger.debug(line.rstrip())

                if once:
                    return 0

                deadline += UPDATE_INTERVAL
                delay = deadline - time.monotonic()
                if delay > 0:
                    sleep_with_stop(delay)
                else:
                    deadline = time.monotonic()
        except AuthError as exc:
            logger.error("认证失败: %s" % exc)
        except (ConnectError, OSError) as exc:
            logger.error("连接异常: %s" % exc)
        except KeyboardInterrupt:
            return 0
        finally:
            if sock is not None:
                try:
                    sock.close()
                except OSError:
                    pass

        if once:
            return 1
        sleep_with_stop(backoff)
        backoff = min(backoff * 2.0, RECONNECT_MAX)

    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="ServerStatus-Plus monitoring client (Linux /proc)"
    )
    parser.add_argument(
        "--once", action="store_true",
        help="采集并发送一次后退出（用于测试）",
    )
    parser.add_argument(
        "--verbose", action="store_true",
        help="输出每次上报的详细内容",
    )
    args = parser.parse_args(argv)

    if not sys.platform.startswith("linux"):
        sys.stderr.write("sergate-client 仅支持 Linux（需要 /proc 和 /sys）\n")
        return 1

    logger = Logger(args.verbose)
    try:
        config = build_config()
    except ConfigError as exc:
        sys.stderr.write("配置错误: %s\n" % exc)
        return 2

    install_signal_handlers()
    return run(config, args.once, logger)


if __name__ == "__main__":
    sys.exit(main())
