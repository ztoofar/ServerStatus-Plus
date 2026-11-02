#[cfg(not(unix))]
fn main() {
    eprintln!("sergate-client requires Linux/Unix");
    std::process::exit(1);
}

#[cfg(unix)]
fn main() {
    imp::run();
}

#[cfg(unix)]
mod imp {
    use clap::Parser;
    use serde::{Deserialize, Serialize};
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::ffi::CString;
    use std::fs;
    use std::io::{self, Read, Write};
    use std::net::{Ipv4Addr, Ipv6Addr, TcpStream, ToSocketAddrs};
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, Instant};

    const EXCLUDED_PREFIXES: &[&str] = &[
        "lo", "docker", "br-", "veth", "virbr", "tun", "tap", "wg", "tailscale", "zt",
        "cni", "flannel", "podman", "lxc", "vmnet", "vnet", "macvlan", "ipvlan", "ifb",
        "dummy",
    ];

    const EXCLUDED_FS: &[&str] = &[
        "proc", "sysfs", "devtmpfs", "tmpfs", "devpts", "cgroup", "cgroup2", "securityfs",
        "debugfs", "tracefs", "configfs", "pstore", "mqueue", "hugetlbfs", "rpc_pipefs",
        "autofs", "overlay", "squashfs", "fusectl", "bpf", "nfs", "nfs4", "cifs", "smb3",
        "sshfs", "9p", "ceph", "glusterfs",
    ];

    #[derive(Debug, Parser)]
    #[command(name = "sergate-client", version, about = "ServerStatus-Plus monitoring client")]
    struct Args {
        #[arg(short, long, default_value = "/usr/local/ServerStatus/client/config.json")]
        config: PathBuf,
    }

    #[derive(Debug, Clone, Deserialize)]
    struct ClientConfig {
        #[serde(default = "default_server")]
        server: String,
        #[serde(default = "default_port")]
        port: u16,
        username: String,
        password: String,
        #[serde(default)]
        family: u8,
        #[serde(default = "default_interval")]
        interval: u64,
        #[serde(default = "default_v4_target")]
        online_check_v4: String,
        #[serde(default = "default_v6_target")]
        online_check_v6: String,
        #[serde(default)]
        interfaces: Vec<String>,
    }

    impl ClientConfig {
        fn load(path: &Path) -> io::Result<Self> {
            let data = fs::read_to_string(path)?;
            serde_json::from_str(&data).map_err(|err| {
                io::Error::new(io::ErrorKind::InvalidData, format!("parse config: {err}"))
            })
        }
    }

    #[derive(Debug, Serialize)]
    struct Payload {
        uptime: i64,
        load: f64,
        memory_total: i64,
        memory_used: i64,
        swap_total: i64,
        swap_used: i64,
        hdd_total: i64,
        hdd_used: i64,
        cpu: f64,
        network_rx: u64,
        network_tx: u64,
        network_in: u64,
        network_out: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        online4: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        online6: Option<bool>,
    }

    struct Monitor<'a> {
        cfg: &'a ClientConfig,
        version: u8,
        cpu_prev: Option<(u64, u64)>,
        network: Network,
        online4: bool,
        online6: bool,
        last_online: Option<Instant>,
    }

    impl<'a> Monitor<'a> {
        fn new(cfg: &'a ClientConfig, version: u8) -> Self {
            Self {
                cfg,
                version,
                cpu_prev: None,
                network: Network::new(scan_interfaces(&cfg.interfaces)),
                online4: false,
                online6: false,
                last_online: None,
            }
        }

        fn update(&mut self) -> Payload {
            let cpu = cpu_usage(&mut self.cpu_prev);
            let (memory_total, memory_used, swap_total, swap_used) = memory();
            let (hdd_total, hdd_used) = disk_usage();
            let (network_rx, network_tx, network_in, network_out) = self.network.update();

            let check_version = if self.version == 4 { 6 } else { 4 };
            if self
                .last_online
                .map_or(true, |last| last.elapsed() >= Duration::from_secs(30))
            {
                let target = if check_version == 4 {
                    &self.cfg.online_check_v4
                } else {
                    &self.cfg.online_check_v6
                };
                let online = check_online(target, check_version);
                if check_version == 4 {
                    self.online4 = online;
                } else {
                    self.online6 = online;
                }
                self.last_online = Some(Instant::now());
            }

            let (online4, online6) = if self.version == 4 {
                (None, Some(self.online6))
            } else {
                (Some(self.online4), None)
            };

            Payload {
                uptime: uptime(),
                load: established_load(),
                memory_total,
                memory_used,
                swap_total,
                swap_used,
                hdd_total,
                hdd_used,
                cpu: cpu.round(),
                network_rx,
                network_tx,
                network_in,
                network_out,
                online4,
                online6,
            }
        }
    }

    pub fn run() {
        let args = Args::parse();
        let config = match ClientConfig::load(&args.config) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("cannot load {}: {err}", args.config.display());
                std::process::exit(1);
            }
        };

        loop {
            if let Err(err) = run_connection(&config) {
                eprintln!("Connection error: {err}");
                thread::sleep(Duration::from_secs(3));
            }
        }
    }

    fn run_connection(cfg: &ClientConfig) -> io::Result<()> {
        let (mut stream, version) = connect(cfg)?;
        println!("Authentication successful");
        let mut monitor = Monitor::new(cfg, version);
        let mut next = Instant::now();

        loop {
            let payload = monitor.update();
            let json = serde_json::to_string(&payload)
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
            stream.write_all(format!("update {json}\n").as_bytes())?;
            stream.flush()?;

            next += Duration::from_secs(cfg.interval.max(1));
            let now = Instant::now();
            if next > now {
                thread::sleep(next - now);
            } else {
                next = now;
            }
        }
    }

    fn connect(cfg: &ClientConfig) -> io::Result<(TcpStream, u8)> {
        let addrs: Vec<_> = (cfg.server.as_str(), cfg.port)
            .to_socket_addrs()?
            .filter(|addr| match cfg.family {
                4 => addr.is_ipv4(),
                6 => addr.is_ipv6(),
                _ => true,
            })
            .collect();

        let mut last_error = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
                Ok(mut stream) => {
                    let _ = stream.set_nodelay(true);
                    stream.set_read_timeout(Some(Duration::from_secs(10)))?;

                    let greeting = read_until(&mut stream, &["Authentication required"])?;
                    if !greeting.contains("Authentication required") {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "authentication request failed",
                        ));
                    }

                    stream.write_all(
                        format!("{}:{}\n", cfg.username, cfg.password).as_bytes(),
                    )?;

                    let response = read_until(
                        &mut stream,
                        &[
                            "IPv4",
                            "IPv6",
                            "Authentication failed",
                            "Only one connection",
                        ],
                    )?;
                    if !response.contains("Authentication successful") {
                        let message = response.trim();
                        return Err(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            if message.is_empty() {
                                "authentication failed".to_string()
                            } else {
                                message.to_string()
                            },
                        ));
                    }

                    let version = if response.contains("IPv6") {
                        6
                    } else if response.contains("IPv4") {
                        4
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "unknown network mode",
                        ));
                    };

                    stream.set_read_timeout(None)?;
                    return Ok((stream, version));
                }
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            io::Error::new(io::ErrorKind::AddrNotAvailable, "no usable address")
        }))
    }

    fn read_until(stream: &mut TcpStream, markers: &[&str]) -> io::Result<String> {
        let mut data = Vec::new();
        let mut buf = [0u8; 1024];
        while data.len() < 4096 {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    data.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&data);
                    if markers.iter().any(|marker| text.contains(marker)) {
                        return Ok(text.into_owned());
                    }
                }
                Err(err)
                    if err.kind() == io::ErrorKind::WouldBlock
                        || err.kind() == io::ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(err) => return Err(err),
            }
        }
        Ok(String::from_utf8_lossy(&data).into_owned())
    }

    fn cpu_usage(previous: &mut Option<(u64, u64)>) -> f64 {
        let Ok(text) = fs::read_to_string("/proc/stat") else {
            return 0.0;
        };
        let Some(line) = text.lines().next() else {
            return 0.0;
        };
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 9 || fields[0] != "cpu" {
            return 0.0;
        }
        let samples: Vec<u64> = fields[1..9]
            .iter()
            .filter_map(|value| value.parse().ok())
            .collect();
        if samples.len() < 8 {
            return 0.0;
        }
        let total: u64 = samples.iter().sum();
        let idle = samples[3] + samples[4];

        let usage = match *previous {
            Some((old_total, old_idle)) => {
                let total_delta = total.saturating_sub(old_total);
                let idle_delta = idle.saturating_sub(old_idle);
                if total_delta == 0 {
                    0.0
                } else {
                    100.0 * (total_delta - idle_delta) as f64 / total_delta as f64
                }
            }
            None => 0.0,
        };
        *previous = Some((total, idle));
        usage.clamp(0.0, 100.0)
    }

    fn memory() -> (i64, i64, i64, i64) {
        let text = fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let mut values = HashMap::new();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let Some(key) = parts.next() else { continue };
            let Some(value) = parts.next() else { continue };
            if let Ok(value) = value.parse::<i64>() {
                values.insert(key.trim_end_matches(':').to_string(), value);
            }
        }

        let total = values.get("MemTotal").copied().unwrap_or(0);
        let fallback_available = values.get("MemFree").copied().unwrap_or(0)
            + values.get("Buffers").copied().unwrap_or(0)
            + values.get("Cached").copied().unwrap_or(0)
            + values.get("SReclaimable").copied().unwrap_or(0)
            - values.get("Shmem").copied().unwrap_or(0);
        let available = values
            .get("MemAvailable")
            .copied()
            .unwrap_or(fallback_available)
            .clamp(0, total);
        let swap_total = values.get("SwapTotal").copied().unwrap_or(0);
        let swap_free = values.get("SwapFree").copied().unwrap_or(0);

        (
            total,
            (total - available).max(0),
            swap_total,
            (swap_total - swap_free).max(0),
        )
    }

    fn uptime() -> i64 {
        fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|text| text.split_whitespace().next()?.parse::<f64>().ok())
            .unwrap_or(0.0) as i64
    }

    fn disk_usage() -> (i64, i64) {
        let text = fs::read_to_string("/proc/self/mounts").unwrap_or_default();
        let mut mounts = Vec::new();
        let mut devices = HashSet::new();

        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 3 || EXCLUDED_FS.contains(&fields[2]) {
                continue;
            }
            let mount = decode_mount(fields[1]);
            if let Ok(metadata) = fs::metadata(&mount) {
                if devices.insert(metadata.dev()) {
                    mounts.push(mount);
                }
            }
        }
        if mounts.is_empty() {
            mounts.push("/".to_string());
        }

        let mut total = 0u64;
        let mut used = 0u64;
        for mount in mounts {
            if let Some((mount_total, mount_used)) = statvfs(&mount) {
                total += mount_total;
                used += mount_used;
            }
        }

        ((total / 1_048_576) as i64, (used / 1_048_576) as i64)
    }

    fn statvfs(path: &str) -> Option<(u64, u64)> {
        let path = CString::new(path).ok()?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(path.as_ptr(), &mut stat) } != 0 {
            return None;
        }
        let total = stat.f_blocks as u64 * stat.f_frsize as u64;
        let free = stat.f_bavail as u64 * stat.f_frsize as u64;
        Some((total, total.saturating_sub(free)))
    }

    fn decode_mount(path: &str) -> String {
        let bytes = path.as_bytes();
        let mut output = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'\\' && index + 3 < bytes.len() {
                let octal = &bytes[index + 1..index + 4];
                if octal.iter().all(|byte| (b'0'..=b'7').contains(byte)) {
                    let value = (octal[0] - b'0') as u32 * 64
                        + (octal[1] - b'0') as u32 * 8
                        + (octal[2] - b'0') as u32;
                    output.push(value as u8);
                    index += 4;
                    continue;
                }
            }
            output.push(bytes[index]);
            index += 1;
        }
        String::from_utf8_lossy(&output).into_owned()
    }

    fn established_load() -> f64 {
        let mut addresses = HashSet::new();
        read_tcp_table("/proc/net/tcp", &mut addresses);
        read_tcp_table("/proc/net/tcp6", &mut addresses);
        addresses.len() as f64
    }

    fn read_tcp_table(path: &str, addresses: &mut HashSet<String>) {
        let text = fs::read_to_string(path).unwrap_or_default();
        for line in text.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 4 || fields[3] != "01" {
                continue;
            }
            let raw = fields[2].split(':').next().unwrap_or("");
            let address = if path.ends_with("/tcp") {
                parse_ipv4_hex(raw).map(|ip| ip.to_string())
            } else {
                parse_ipv6_hex(raw)
            };
            if let Some(address) = address {
                addresses.insert(address);
            }
        }
    }

    fn parse_ipv4_hex(raw: &str) -> Option<Ipv4Addr> {
        let bytes = hex_bytes(raw)?;
        if bytes.len() != 4 {
            return None;
        }
        Some(Ipv4Addr::new(bytes[3], bytes[2], bytes[1], bytes[0]))
    }

    fn parse_ipv6_hex(raw: &str) -> Option<String> {
        let bytes = hex_bytes(raw)?;
        if bytes.len() != 16 {
            return None;
        }
        let mut data = [0u8; 16];
        for word in 0..4 {
            for byte in 0..4 {
                data[word * 4 + byte] = bytes[word * 4 + (3 - byte)];
            }
        }
        let ip = Ipv6Addr::from(data);
        Some(match ip.to_ipv4_mapped() {
            Some(ipv4) => ipv4.to_string(),
            None => ip.to_string(),
        })
    }

    fn hex_bytes(raw: &str) -> Option<Vec<u8>> {
        if raw.len() % 2 != 0 {
            return None;
        }
        (0..raw.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&raw[index..index + 2], 16).ok())
            .collect()
    }

    struct Network {
        names: Vec<String>,
        previous: Option<(Instant, u64, u64)>,
        rx_history: VecDeque<u64>,
        tx_history: VecDeque<u64>,
    }

    impl Network {
        fn new(names: Vec<String>) -> Self {
            Self {
                names,
                previous: None,
                rx_history: VecDeque::with_capacity(5),
                tx_history: VecDeque::with_capacity(5),
            }
        }

        fn update(&mut self) -> (u64, u64, u64, u64) {
            let (rx_total, tx_total) = read_network_counters(&self.names);
            let now = Instant::now();

            let (rx_speed, tx_speed) = match self.previous {
                Some((previous_time, previous_rx, previous_tx)) => {
                    let elapsed = now.duration_since(previous_time).as_secs_f64();
                    if elapsed > 0.0 {
                        let rx = ((rx_total.saturating_sub(previous_rx)) as f64 / elapsed) as u64;
                        let tx = ((tx_total.saturating_sub(previous_tx)) as f64 / elapsed) as u64;
                        push_history(&mut self.rx_history, rx);
                        push_history(&mut self.tx_history, tx);
                        (
                            self.rx_history.iter().sum::<u64>() / self.rx_history.len() as u64,
                            self.tx_history.iter().sum::<u64>() / self.tx_history.len() as u64,
                        )
                    } else {
                        (0, 0)
                    }
                }
                None => (0, 0),
            };
            self.previous = Some((now, rx_total, tx_total));

            (rx_speed, tx_speed, rx_total, tx_total)
        }
    }

    fn push_history(history: &mut VecDeque<u64>, value: u64) {
        if history.len() == 5 {
            history.pop_front();
        }
        history.push_back(value);
    }

    fn scan_interfaces(configured: &[String]) -> Vec<String> {
        if !configured.is_empty() {
            return configured.to_vec();
        }
        let Ok(entries) = fs::read_dir("/sys/class/net") else {
            return Vec::new();
        };
        entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| is_physical_interface(name))
            .collect()
    }

    fn is_physical_interface(name: &str) -> bool {
        if EXCLUDED_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
        {
            return false;
        }
        let base = format!("/sys/class/net/{name}");
        let type_ok = fs::read_to_string(format!("{base}/type"))
            .ok()
            .map(|value| value.trim() == "1")
            .unwrap_or(false);
        if !type_ok || !Path::new(&format!("{base}/device")).exists() {
            return false;
        }
        let ifindex = fs::read_to_string(format!("{base}/ifindex"))
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok());
        let iflink = fs::read_to_string(format!("{base}/iflink"))
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok());
        matches!((ifindex, iflink), (Some(a), Some(b)) if a == b)
    }

    fn read_network_counters(names: &[String]) -> (u64, u64) {
        let text = fs::read_to_string("/proc/net/dev").unwrap_or_default();
        let mut rx = 0u64;
        let mut tx = 0u64;
        for line in text.lines().skip(2) {
            let Some((name, values)) = line.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if names.is_empty() {
                if name == "lo" || name.starts_with("tun") {
                    continue;
                }
            } else if !names.iter().any(|item| item == name) {
                continue;
            }
            let fields: Vec<&str> = values.split_whitespace().collect();
            if fields.len() < 16 {
                continue;
            }
            if let (Ok(rx_value), Ok(tx_value)) =
                (fields[0].parse::<u64>(), fields[8].parse::<u64>())
            {
                rx = rx.saturating_add(rx_value);
                tx = tx.saturating_add(tx_value);
            }
        }
        (rx, tx)
    }

    fn check_online(target: &str, family: u8) -> bool {
        let Ok(addresses) = target.to_socket_addrs() else {
            return false;
        };
        for address in addresses {
            if family == 4 && !address.is_ipv4() {
                continue;
            }
            if family == 6 && !address.is_ipv6() {
                continue;
            }
            if TcpStream::connect_timeout(&address, Duration::from_secs(2)).is_ok() {
                return true;
            }
        }
        false
    }

    fn default_server() -> String {
        "127.0.0.1".to_string()
    }

    fn default_port() -> u16 {
        35601
    }

    fn default_interval() -> u64 {
        1
    }

    fn default_v4_target() -> String {
        "www.aliyun.com:443".to_string()
    }

    fn default_v6_target() -> String {
        "www.aliyun.com:443".to_string()
    }
}
