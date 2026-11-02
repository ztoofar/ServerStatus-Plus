#!/usr/bin/env python3
"""Linux-only integration test for client/sergate-client.py.

Runs a local fake TCP server, executes the client with ``--once``, and checks
the authentication line and reported payload. Pure helper coverage lives in
tests/test_client.py and runs on every platform.
"""

import json
import os
import socket
import subprocess
import sys
import tempfile
import threading
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLIENT_PATH = os.path.join(ROOT, "client", "sergate-client.py")


def recv_line(conn):
    data = bytearray()
    while b"\n" not in data and len(data) < 65536:
        chunk = conn.recv(4096)
        if not chunk:
            break
        data.extend(chunk)
    return bytes(data).split(b"\n", 1)[0].decode("utf-8", "replace")


class FakeServer(object):
    def __init__(self, family):
        self.family = family
        self.result = {}
        self.error = None
        self.sock = socket.socket(family, socket.SOCK_STREAM)
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        if family == socket.AF_INET6:
            self.sock.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 1)
            self.sock.bind(("::1", 0))
            self.host = "::1"
        else:
            self.sock.bind(("127.0.0.1", 0))
            self.host = "127.0.0.1"
        self.port = self.sock.getsockname()[1]
        self.sock.listen(1)
        self.thread = threading.Thread(target=self._serve)
        self.thread.daemon = True

    def start(self):
        self.thread.start()
        return self

    def _serve(self):
        try:
            conn, _addr = self.sock.accept()
            try:
                conn.settimeout(10)
                conn.sendall(b"Authentication required:\n")
                self.result["auth"] = recv_line(conn)
                mode = b"IPv6" if self.family == socket.AF_INET6 else b"IPv4"
                conn.sendall(
                    b"Authentication successful. Access granted.\n"
                    b"You are connecting via: " + mode + b"\n"
                )
                self.result["line"] = recv_line(conn)
            finally:
                conn.close()
        except Exception as exc:  # pragma: no cover - surfaced in the test
            self.error = exc
        finally:
            self.sock.close()

    def join(self):
        self.thread.join(timeout=20)
        if self.error is not None:
            raise self.error


def write_config(host, port, family):
    handle = tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", suffix=".json", delete=False
    )
    try:
        json.dump({
            "server": host,
            "port": port,
            "username": "user",
            "password": "pass",
            "family": family,
            "interval": 1,
            "online_check_v4": "127.0.0.1:9",
            "online_check_v6": "127.0.0.1:9",
            "interfaces": [],
        }, handle)
    finally:
        handle.close()
    return handle.name


@unittest.skipUnless(sys.platform.startswith("linux"), "client requires Linux /proc")
class ClientSmokeTests(unittest.TestCase):
    def run_case(self, server_family, client_family):
        try:
            server = FakeServer(server_family).start()
        except OSError as exc:
            self.skipTest("cannot bind test server: %s" % exc)

        config_path = write_config(server.host, server.port, client_family)
        self.addCleanup(os.unlink, config_path)
        try:
            completed = subprocess.run(
                [sys.executable, CLIENT_PATH, "-c", config_path, "--once"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=20,
            )
            server.join()
        finally:
            if server.thread.is_alive():
                server.thread.join(timeout=1)

        self.assertEqual(
            completed.returncode,
            0,
            "stdout=%r stderr=%r"
            % (
                completed.stdout.decode("utf-8", "replace"),
                completed.stderr.decode("utf-8", "replace"),
            ),
        )
        self.assertEqual(server.result.get("auth"), "user:pass")
        line = server.result.get("line", "")
        self.assertTrue(line.startswith("update "), line)
        payload = json.loads(line[len("update "):])
        for field in (
            "uptime", "load", "memory_total", "memory_used", "swap_total",
            "swap_used", "hdd_total", "hdd_used", "cpu", "network_rx",
            "network_tx", "network_in", "network_out",
        ):
            self.assertIn(field, payload)
            self.assertGreaterEqual(payload[field], 0, field)
        self.assertIsInstance(payload["cpu"], int)
        if server_family == socket.AF_INET6:
            self.assertIn("online4", payload)
            self.assertNotIn("online6", payload)
        else:
            self.assertIn("online6", payload)
            self.assertNotIn("online4", payload)

    def test_ipv4(self):
        self.run_case(socket.AF_INET, 4)

    def test_ipv6(self):
        if not socket.has_ipv6:
            self.skipTest("IPv6 unavailable")
        self.run_case(socket.AF_INET6, 6)

    def test_auto_family_prefers_available(self):
        self.run_case(socket.AF_INET, 0)


if __name__ == "__main__":
    unittest.main()
