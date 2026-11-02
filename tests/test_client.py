#!/usr/bin/env python3
"""Unit tests for pure helpers in client/sergate-client.py."""

import importlib.util
import json
import os
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLIENT_PATH = os.path.join(ROOT, "client", "sergate-client.py")

_spec = importlib.util.spec_from_file_location("sergate_client", CLIENT_PATH)
client = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(client)


class MetricHelperTests(unittest.TestCase):
    def test_parse_meminfo(self):
        values = client.parse_meminfo([
            "MemTotal:       2048000 kB\n",
            "MemAvailable:    512000 kB\n",
            "SwapTotal:       1048576 kB\n",
            "SwapFree:         524288 kB\n",
        ])
        self.assertEqual(values["MemTotal"], 2048000)
        self.assertEqual(values["MemAvailable"], 512000)
        self.assertEqual(values["SwapFree"], 524288)

    def test_memory_values_available(self):
        values = {
            "MemTotal": 1000,
            "MemAvailable": 400,
            "SwapTotal": 100,
            "SwapFree": 40,
        }
        self.assertEqual(client.memory_values(values), (1000, 600, 100, 60))

    def test_memory_values_fallback(self):
        values = {
            "MemTotal": 1000,
            "MemFree": 100,
            "Buffers": 50,
            "Cached": 150,
            "SReclaimable": 20,
            "Shmem": 20,
        }
        total, used, _swap_total, _swap_used = client.memory_values(values)
        self.assertEqual(total, 1000)
        self.assertEqual(used, 700)

    def test_decode_mount(self):
        self.assertEqual(client.decode_mount(r"/mnt/disk\040one"), "/mnt/disk one")
        self.assertEqual(client.decode_mount(r"/mnt/tab\011x"), "/mnt/tab\tx")
        self.assertEqual(client.decode_mount("/plain"), "/plain")

    def test_decode_tcp_address_ipv4(self):
        self.assertEqual(client.decode_tcp_address("0100007F", False), "127.0.0.1")

    def test_decode_tcp_address_ipv6(self):
        self.assertEqual(
            client.decode_tcp_address("0" * 32, True),
            "::",
        )
        self.assertEqual(
            client.decode_tcp_address("0000000000000000ffff00000100007f", True),
            "127.0.0.1",
        )

    def test_parse_tcp_table_unique_established(self):
        lines = [
            "  sl  local_address rem_address   st\n",
            "   0: 0100007F:1F90 0100007F:D431 01\n",
            "   1: 0100007F:1F90 0200007F:D431 01\n",
            "   2: 0100007F:1F90 0300007F:D431 0A\n",
        ]
        addresses = set()
        client.parse_tcp_table(lines, False, addresses)
        self.assertEqual(addresses, {"127.0.0.1", "127.0.0.2"})

    def test_parse_loadavg(self):
        self.assertEqual(client.parse_loadavg("0.12 0.34 0.56 1/123 4567\n"), 0.12)
        self.assertEqual(client.parse_loadavg("1.5 1.2 0.9 2/100 9999\n"), 1.5)
        self.assertEqual(client.parse_loadavg(""), 0.0)

    def test_parse_target(self):
        self.assertEqual(
            client.parse_target("www.example.com:443"), ("www.example.com", 443)
        )
        self.assertEqual(
            client.parse_target("[2001:db8::1]:8443"), ("2001:db8::1", 8443)
        )
        self.assertEqual(client.parse_target("2001:db8::1"), ("2001:db8::1", 80))
        self.assertEqual(client.parse_target("example.com"), ("example.com", 80))
        self.assertEqual(client.parse_target(""), ("", 0))
        self.assertEqual(client.parse_target("host:bad"), ("", 0))


class ConfigTests(unittest.TestCase):
    def write_config(self, payload):
        handle = tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", suffix=".json", delete=False
        )
        try:
            json.dump(payload, handle)
        finally:
            handle.close()
        self.addCleanup(os.unlink, handle.name)
        return handle.name

    def test_defaults(self):
        path = self.write_config({
            "server": "127.0.0.1",
            "port": 35601,
            "username": "s01",
            "password": "pass",
        })
        config = client.load_config(path)
        self.assertEqual(config.family, 0)
        self.assertEqual(config.interval, 1)
        self.assertEqual(config.load_mode, "loadavg")
        self.assertEqual(config.online_check_v4, "www.aliyun.com:443")
        self.assertEqual(config.online_check_v6, "www.aliyun.com:443")
        self.assertEqual(config.interfaces, [])

    def test_missing_password(self):
        path = self.write_config({
            "server": "127.0.0.1",
            "username": "s01",
        })
        with self.assertRaises(client.ConfigError):
            client.load_config(path)

    def test_interval_too_long(self):
        path = self.write_config({
            "server": "127.0.0.1",
            "username": "s01",
            "password": "pass",
            "interval": 60,
        })
        with self.assertRaises(client.ConfigError):
            client.load_config(path)

    def test_family_filter(self):
        path = self.write_config({
            "server": "127.0.0.1",
            "username": "s01",
            "password": "pass",
            "family": 6,
        })
        self.assertEqual(client.load_config(path).family, 6)

    def test_load_mode_connections(self):
        path = self.write_config({
            "server": "127.0.0.1",
            "username": "s01",
            "password": "pass",
            "load_mode": "connections",
        })
        self.assertEqual(client.load_config(path).load_mode, "connections")

    def test_invalid_load_mode(self):
        path = self.write_config({
            "server": "127.0.0.1",
            "username": "s01",
            "password": "pass",
            "load_mode": "bogus",
        })
        with self.assertRaises(client.ConfigError):
            client.load_config(path)


if __name__ == "__main__":
    unittest.main()
