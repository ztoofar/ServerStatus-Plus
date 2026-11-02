#!/usr/bin/env python3
"""Unit tests for pure helpers in client/sergate-client.py."""

import importlib.util
import os
import stat
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

    def test_memory_values_exclude_cache(self):
        # Same accounting as free(1) and the upstream ServerStatus client.
        values = {
            "MemTotal": 1000,
            "MemFree": 100,
            "Buffers": 50,
            "Cached": 150,
            "SReclaimable": 20,
            "MemAvailable": 400,
            "SwapTotal": 100,
            "SwapFree": 40,
        }
        self.assertEqual(client.memory_values(values), (1000, 680, 100, 60))

    def test_memory_values_clamped(self):
        values = {
            "MemTotal": 1000,
            "MemFree": 900,
            "Buffers": 100,
            "Cached": 100,
            "SReclaimable": 100,
        }
        _total, used, swap_total, swap_used = client.memory_values(values)
        self.assertEqual(used, 0)
        self.assertEqual(swap_total, 0)
        self.assertEqual(swap_used, 0)

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

    def test_is_local_address(self):
        self.assertTrue(client.is_local_address("127.0.0.1"))
        self.assertTrue(client.is_local_address("127.0.0.2"))
        self.assertTrue(client.is_local_address("::1"))
        self.assertTrue(client.is_local_address("::"))
        self.assertFalse(client.is_local_address("192.168.1.2"))
        self.assertFalse(client.is_local_address("2001:db8::1"))

    def test_parse_tcp_table_unique_remote(self):
        ipv4_lines = [
            "  sl  local_address rem_address   st\n",
            "   0: 0100007F:1F90 0100007F:D431 01\n",
            "   1: 0100007F:1F90 0201A8C0:D432 01\n",
            "   2: 0100007F:1F90 0201A8C0:D433 01\n",
            "   3: 0100007F:1F90 0100000A:D434 01\n",
            "   4: 0100007F:1F90 0100000A:D435 0A\n",
        ]
        addresses = set()
        client.parse_tcp_table(ipv4_lines, False, addresses)
        self.assertEqual(addresses, {"192.168.1.2", "10.0.0.1"})

        ipv6_lines = [
            "  sl  local_address rem_address   st\n",
            "   0: 00000000000000000000000000000000:1F90 "
            "0000000000000000ffff00000100007f:D431 01\n",
            "   1: 00000000000000000000000000000000:1F90 "
            "0000000000000000ffff00000301a8c0:D432 01\n",
            "   2: 00000000000000000000000000000000:1F90 "
            "b80d0120000000000000000001000000:D433 01\n",
            "   3: 00000000000000000000000000000000:1F90 "
            "00000000000000000000000000000000:D434 01\n",
        ]
        client.parse_tcp_table(ipv6_lines, True, addresses)
        self.assertEqual(
            addresses,
            {"192.168.1.2", "10.0.0.1", "192.168.1.3", "2001:db8::1"},
        )


class FakeStat(object):
    def __init__(self, mode=0, dev=0, rdev=0):
        self.st_mode = mode
        self.st_dev = dev
        self.st_rdev = rdev


class MountSelectionTests(unittest.TestCase):
    def test_btrfs_subvolumes_counted_once(self):
        # Synology mounts several subvolumes of one btrfs filesystem; each
        # has its own st_dev but shares capacity with the parent volume.
        block = stat.S_IFBLK | 0o600
        known = {
            "/dev/mapper/cachedev_0": FakeStat(block, rdev=0xFD00),
            "/dev/md0": FakeStat(block, rdev=0x0900),
            "/volume1": FakeStat(dev=0x1000),
            "/volume1/@docker/btrfs": FakeStat(dev=0x1001),
            "/volume1/Data/#snapshot": FakeStat(dev=0x1002),
            "/": FakeStat(dev=0x2000),
        }

        def fake_stat(path):
            if path in known:
                return known[path]
            raise OSError(path)

        lines = [
            "/dev/mapper/cachedev_0 /volume1 btrfs rw,subvol=/@syno 0 0\n",
            "/dev/mapper/cachedev_0 /volume1/@docker/btrfs btrfs "
            "rw,subvol=/@syno/@docker/btrfs 0 0\n",
            "/dev/mapper/cachedev_0 /volume1/Data/#snapshot btrfs "
            "ro,subvol=/@syno/@sharesnap/Data 0 0\n",
            "/dev/md0 / ext4 rw 0 0\n",
            "tmpfs /run tmpfs rw 0 0\n",
        ]

        self.assertEqual(
            client.select_mounts(lines, fake_stat),
            ["/volume1", "/"],
        )

    def test_bind_mounts_with_unknown_source_deduped_by_st_dev(self):
        known = {
            "/": FakeStat(dev=0x2000),
            "/srv/bind": FakeStat(dev=0x2000),
        }

        def fake_stat(path):
            if path in known:
                return known[path]
            raise OSError(path)

        lines = [
            "/dev/root / ext4 rw 0 0\n",
            "/dev/root /srv/bind ext4 rw 0 0\n",
        ]

        self.assertEqual(client.select_mounts(lines, fake_stat), ["/"])


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
