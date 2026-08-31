use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket as StdUdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::time;

#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub last_seen: Instant,
}

/// The broadcast payload sent over UDP.
#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryMessage {
    device_name: String,
    ip: String,
    port: u16,
    timestamp: u64,
}

pub fn get_local_ip() -> String {
    let socket = StdUdpSocket::bind("0.0.0.0:0").ok();
    if let Some(s) = socket {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}
fn get_broadcast_address(local_ip: &str) -> Ipv4Addr {
    if let Ok(ip) = local_ip.parse::<Ipv4Addr>() {
        let octets = ip.octets();
        let broadcast = Ipv4Addr::new(octets[0], octets[1], octets[2], 255);
        tracing::info!("Using subnet broadcast address: {broadcast}");
        return broadcast;
    }
    tracing::warn!("Could not parse local IP '{local_ip}', falling back to 255.255.255.255");
    Ipv4Addr::BROADCAST
}

pub fn spawn_broadcast(
    handle: &tokio::runtime::Handle,
    device_name: String,
    port: u16,
    discovery_port: u16,
    ctx: egui::Context,
) {
    let local_ip = get_local_ip();
    handle.spawn(async move {
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to bind broadcast socket: {e}");
                return;
            }
        };
        if let Err(e) = socket.set_broadcast(true) {
            tracing::error!("Failed to set broadcast: {e}");
            return;
        }
        let subnet_broadcast = get_broadcast_address(&local_ip);
        let broadcast_addr = SocketAddr::V4(SocketAddrV4::new(
            subnet_broadcast,
            discovery_port,
        ));
        let limited_broadcast_addr = SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::BROADCAST,
            discovery_port,
        ));

        tracing::info!(
            "Broadcasting as '{}' on {} (port {}) -> broadcast {}",
            device_name, local_ip, port, subnet_broadcast
        );

        loop {
            let msg = DiscoveryMessage {
                device_name: device_name.clone(),
                ip: local_ip.clone(),
                port,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            if let Ok(data) = serde_json::to_vec(&msg) {
                if let Err(e) = socket.send_to(&data, broadcast_addr).await {
                    tracing::warn!("Subnet broadcast send error: {e}");
                }
                if subnet_broadcast != Ipv4Addr::BROADCAST {
                    if let Err(e) = socket.send_to(&data, limited_broadcast_addr).await {
                        tracing::warn!("Limited broadcast send error: {e}");
                    }
                }
            }

            time::sleep(Duration::from_secs(2)).await;
            ctx.request_repaint();
        }
    });
}

pub fn spawn_listener(
    handle: &tokio::runtime::Handle,
    discovery_port: u16,
    devices: Arc<Mutex<Vec<DiscoveredDevice>>>,
    ctx: egui::Context,
) {
    let local_ip = get_local_ip();
    handle.spawn(async move {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, discovery_port));
        let std_socket = match StdUdpSocket::bind(addr) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to bind discovery listener on port {discovery_port}: {e}");
                return;
            }
        };
        std_socket.set_nonblocking(true).ok();

        let socket = match UdpSocket::from_std(std_socket) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to convert to tokio socket: {e}");
                return;
            }
        };

        tracing::info!("Discovery listener started on port {discovery_port} (local IP: {local_ip})");

        let mut buf = vec![0u8; 4096];
        loop {
            {
                let mut devs = devices.lock().unwrap();
                devs.retain(|d| d.last_seen.elapsed() < Duration::from_secs(10));
            }

            match tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf)).await {
                Ok(Ok((len, src_addr))) => {
                    if let Ok(msg) = serde_json::from_slice::<DiscoveryMessage>(&buf[..len]) {
                        let src_ip_str = src_addr.ip().to_string();
                        if msg.ip == local_ip || src_ip_str == local_ip {
                            continue;
                        }

                        let mut devs = devices.lock().unwrap();
                        if let Some(existing) = devs.iter_mut().find(|d| d.ip == msg.ip && d.port == msg.port) {
                            existing.name = msg.device_name;
                            existing.last_seen = Instant::now();
                        } else {
                            tracing::info!(
                                "Discovered device: {} at {}:{} (source: {})",
                                msg.device_name, msg.ip, msg.port, src_ip_str
                            );
                            devs.push(DiscoveredDevice {
                                name: msg.device_name,
                                ip: msg.ip,
                                port: msg.port,
                                last_seen: Instant::now(),
                            });
                        }
                        ctx.request_repaint();
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("Discovery recv error: {e}");
                }
                Err(_) => {
                }
            }
        }
    });
}
