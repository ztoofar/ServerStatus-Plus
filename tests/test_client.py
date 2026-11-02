#!/usr/bin/env python3
"""Unit tests for pure helpers in client/sergate-client.py."""

import importlib.util
import os
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
        self.assertEqual(client.decode_tcp_address("0" * 32, True), "::")
        self.assertEqual(
            client.decode_tcp_address("0000000000000000ffff00000100007f", True),
            "127.0.0.1",
        )

    def test_decode_tcp6_ipv4(self):
        self.assertEqual(
            client.decode_tcp6_ipv4("0000000000000000ffff00000100007f"),
            "127.0.0.1",
        )
        self.assertIsNone(client.decode_tcp6_ipv4("0" * 32))

    def test_parse_tcp6_ipv4_unique_established(self):
        lines = [
            "  sl  local_address rem_address   st\n",
            "   0: 00000000000000000000000000000000:1F90 "
            "0000000000000000ffff00000100007f:D431 01\n",
            "   1: 00000000000000000000000000000000:1F90 "
            "0000000000000000ffff00000100007f:D432 01\n",
            "   2: 00000000000000000000000000000000:1F90 "
            "00000000000000000000000000000000:D433 01\n",
            "   3: 00000000000000000000000000000000:1F90 "
            "0000000000000000ffff00000100007f:D434 0A\n",
        ]
        addresses = set()
        client.parse_tcp6_ipv4(lines, addresses)
        self.assertEqual(addresses, {"127.0.0.1"})


class ConfigTests(unittest.TestCase):
    def test_defaults(self):
        config = client.make_config()
        self.assertEqual(config.server, "127.0.0.1")
        self.assertEqual(config.port, 35601)
        self.assertEqual(config.username, "s01")
        self.assertEqual(config.password, "password")

    def test_missing_password(self):
        with self.assertRaises(client.ConfigError):
            client.make_config(password="")

    def test_invalid_port(self):
        with self.assertRaises(client.ConfigError):
            client.make_config(port=70000)


if __name__ == "__main__":
    unittest.main()
