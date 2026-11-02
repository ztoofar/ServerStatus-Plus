#!/usr/bin/env python3
"""End-to-end smoke test for the ServerStatus-Plus Rust backend."""

import json
import socket
import time
import urllib.request

TCP_PORT = 35601
HTTP = "http://127.0.0.1:18080"


def recv_until(sock, marker, limit=8192):
    data = b""
    while marker.encode() not in data and len(data) < limit:
        chunk = sock.recv(1024)
        if not chunk:
            break
        data += chunk
    return data.decode("utf-8", "replace")


def tcp_connect(host, family=socket.AF_INET):
    sock = socket.socket(family, socket.SOCK_STREAM)
    sock.settimeout(5)
    sock.connect((host, TCP_PORT))
    return sock


def api(path):
    with urllib.request.urlopen(HTTP + path, timeout=5) as response:
        return json.loads(response.read().decode("utf-8"))


def login(host, family=socket.AF_INET, username="s01", password="pass"):
    sock = tcp_connect(host, family)
    greeting = recv_until(sock, "Authentication required")
    assert "Authentication required" in greeting, greeting
    sock.sendall(f"{username}:{password}\n".encode())
    response = recv_until(sock, "IPv4" if family == socket.AF_INET else "IPv6")
    return sock, response


def update_payload(online_field):
    return {
        "uptime": 3600,
        "load": 0.5,
        "network_rx": 1024,
        "network_tx": 2048,
        "network_in": 1048576,
        "network_out": 2097152,
        "memory_total": 8192000,
        "memory_used": 4096000,
        "swap_total": 1048576,
        "swap_used": 524288,
        "hdd_total": 40960,
        "hdd_used": 20480,
        "cpu": 12.5,
        online_field: True,
        "custom": "smoke test",
    }


def main():
    # IPv4 connection reports the IPv6 reachability field.
    sock, response = login("127.0.0.1")
    assert "Authentication successful" in response, response
    assert "IPv4" in response, response

    sock.sendall(("update " + json.dumps(update_payload("online6")) + "\n").encode())
    time.sleep(0.4)

    status = api("/api/status")
    assert len(status["servers"]) == 1, status
    node = status["servers"][0]
    assert node["id"] == "s01", node
    assert node["connected"] is True, node
    assert node["connected_via"] == "v4", node
    assert node["online4"] is True, node
    assert node["online6"] is True, node
    assert node["memory_total"] == 8192000, node
    assert node["cpu"] == 12.5, node

    legacy = api("/json/stats.json")
    assert legacy["servers"][0]["uptime"] == "01:00:00", legacy

    with urllib.request.urlopen(HTTP + "/api/events", timeout=5) as response:
        first = response.readline().decode("utf-8")
    assert first.startswith("data: "), first

    with urllib.request.urlopen(HTTP + "/", timeout=5) as response:
        html = response.read().decode("utf-8")
    assert "ServerStatus" in html, html[:200]
    assert "/assets/" in html, html[:400]

    sock.close()
    time.sleep(0.5)
    node = api("/api/status")["servers"][0]
    assert node["connected"] is False, node
    assert node["online4"] is False and node["online6"] is False, node

    # IPv6 connection reports the IPv4 reachability field.
    sock6, response6 = login("::1", socket.AF_INET6)
    assert "Authentication successful" in response6, response6
    assert "IPv6" in response6, response6
    sock6.sendall(("update " + json.dumps(update_payload("online4")) + "\n").encode())
    time.sleep(0.4)
    node = api("/api/status")["servers"][0]
    assert node["connected_via"] == "v6", node
    assert node["online6"] is True, node
    assert node["online4"] is True, node
    sock6.close()

    # Wrong password must be rejected.
    sock_bad = tcp_connect("127.0.0.1")
    recv_until(sock_bad, "Authentication required")
    sock_bad.sendall(b"s01:wrong\n")
    bad = recv_until(sock_bad, "Authentication failed")
    assert "Authentication failed" in bad, bad
    sock_bad.close()

    print("smoke test OK")


if __name__ == "__main__":
    main()
