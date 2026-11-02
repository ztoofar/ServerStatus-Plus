#!/usr/bin/env python3
"""End-to-end smoke test for the ServerStatus-Plus Rust backend."""

import http.client
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


def fetch(path):
    with urllib.request.urlopen(HTTP + path, timeout=5) as response:
        return response.read(), response.headers


def api(path):
    body, _headers = fetch(path)
    return json.loads(body.decode("utf-8"))


def raw_request(method, path):
    connection = http.client.HTTPConnection("127.0.0.1", 18080, timeout=5)
    try:
        connection.request(method, path)
        response = connection.getresponse()
        body = response.read()
        return response.status, dict(response.getheaders()), body
    finally:
        connection.close()


def header(headers, name):
    wanted = name.lower()
    for key, value in headers.items():
        if key.lower() == wanted:
            return value
    return None


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
    assert isinstance(legacy["updated"], str), legacy
    assert legacy["servers"][0]["uptime"] == "01:00:00", legacy
    assert legacy["servers"][0]["cpu"] == 12, legacy
    assert isinstance(legacy["servers"][0]["cpu"], int), legacy

    with urllib.request.urlopen(HTTP + "/api/events", timeout=5) as response:
        first = response.readline().decode("utf-8")
    assert first.startswith("data: "), first

    # Original static frontend.
    body, headers = fetch("/")
    html = body.decode("utf-8")
    assert headers.get("Content-Type", "").startswith("text/html"), headers
    assert "no-store" in headers.get("Cache-Control", ""), headers
    assert 'id="servers"' in html, html[:200]
    assert "css/light.css" in html, html[:400]
    assert "js/serverstatus.js" in html, html[:400]
    assert "/assets/" not in html, html[:400]
    assert "{{SITE_" not in html, html[:400]
    assert status["site"]["title"] in html, status["site"]
    if status["site"]["announcement"]:
        assert status["site"]["announcement"] in html, status["site"]
    else:
        assert "欢迎使用 ServerStatus-Plus" in html, status["site"]
    assert "fucking" not in html, html[:400]

    body, headers = fetch("/css/light.css")
    assert headers.get("Content-Type") == "text/css; charset=utf-8", headers
    assert "no-cache" in headers.get("Cache-Control", ""), headers
    assert b"background: #ebebeb" in body

    body, headers = fetch("/js/serverstatus.js")
    js = body.decode("utf-8")
    assert headers.get("Content-Type") == "application/javascript; charset=utf-8", headers
    assert "setInterval(uptime, 2000)" in js
    assert "textContent" in js
    assert "fuck" not in js

    body, headers = fetch("/img/light.png")
    assert headers.get("Content-Type") == "image/png", headers
    assert body.startswith(b"\x89PNG"), body[:8]

    body, headers = fetch("/favicon.ico")
    assert headers.get("Content-Type", "").startswith("image/"), headers
    assert len(body) > 0

    body, headers = fetch("/robots.txt")
    assert headers.get("Content-Type", "").startswith("text/plain"), headers

    body, headers = fetch("/json/stats.json")
    assert "no-store" in headers.get("Cache-Control", ""), headers
    assert headers.get("Pragma") == "no-cache", headers
    assert headers.get("Expires") == "0", headers
    assert body.startswith(b"{"), body[:20]

    # Strict static behavior: no SPA fallback, dotfiles and traversal blocked.
    for path in ("/js/nope.js", "/json/.gitignore", "/.gitignore", "/api/unknown"):
        status_code, _headers, _body = raw_request("GET", path)
        assert status_code == 404, (path, status_code)
    for path in ("/%2e%2e/Cargo.toml", "/../Cargo.toml"):
        status_code, _headers, _body = raw_request("GET", path)
        assert status_code in (400, 404), (path, status_code)

    status_code, headers, _body = raw_request("POST", "/")
    assert status_code == 405, status_code
    assert header(headers, "Allow") == "GET, HEAD", headers

    sock.close()
    time.sleep(0.5)
    node = api("/api/status")["servers"][0]
    assert node["connected"] is False, node
    assert node["online4"] is False and node["online6"] is False, node

    offline = api("/json/stats.json")["servers"][0]
    assert set(offline) == {
        "name", "type", "host", "location", "online4", "online6",
    }, offline
    assert offline["online4"] is False and offline["online6"] is False, offline

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
