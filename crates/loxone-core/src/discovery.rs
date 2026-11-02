//! Local network search for Miniservers.
//!
//! A UDP datagram is broadcast to port 7070 and replies are collected on the
//! sending socket and on port 7071 for a short time. The reply text format is
//! not formally documented; it is parsed loosely (IPv4 `addr[:port]`, a
//! 12-hex-digit serial number, and the remaining words as the name). The raw
//! reply is always kept so nothing is lost if parsing guesses wrong.

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredMiniserver {
    pub address: String,
    pub name: String,
    pub serial: String,
    pub raw: String,
}

fn is_ipv4_with_port(tok: &str) -> bool {
    let host = tok.split(':').next().unwrap_or_default();
    let parts: Vec<&str> = host.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok())
}

fn is_serial(tok: &str) -> bool {
    let t = tok.replace(':', "");
    t.len() == 12 && t.chars().all(|c| c.is_ascii_hexdigit())
}

/// Parses one reply datagram. `from` is the sender address, used as fallback.
pub fn parse_discovery_reply(text: &str, from: &str) -> DiscoveredMiniserver {
    let clean = text.trim_matches(char::from(0)).trim();
    let body = clean
        .strip_prefix("LoxLIVE:")
        .or_else(|| clean.strip_prefix("LoxLIVE"))
        .unwrap_or(clean)
        .trim();
    let mut address = String::new();
    let mut serial = String::new();
    let mut name_parts = Vec::new();
    for tok in body.split_whitespace() {
        if address.is_empty() && is_ipv4_with_port(tok) {
            address = tok.to_string();
        } else if serial.is_empty() && is_serial(tok) {
            serial = tok.replace(':', "").to_uppercase();
        } else if address.is_empty() {
            name_parts.push(tok);
        }
    }
    if address.is_empty() {
        address = from.to_string();
    }
    DiscoveredMiniserver {
        address,
        name: if name_parts.is_empty() {
            "Miniserver".to_string()
        } else {
            name_parts.join(" ")
        },
        serial,
        raw: clean.to_string(),
    }
}

/// Broadcasts a search request and returns all unique replies within `timeout`.
pub fn discover_miniservers(timeout: Duration) -> Result<Vec<DiscoveredMiniserver>, String> {
    let primary = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("UDP bind failed: {}", e))?;
    primary
        .set_broadcast(true)
        .map_err(|e| format!("Broadcast not permitted: {}", e))?;
    primary
        .set_read_timeout(Some(Duration::from_millis(150)))
        .map_err(|e| e.to_string())?;
    // Replies may also be sent to the well-known reply port; listen there when free.
    let reply_port = UdpSocket::bind("0.0.0.0:7071").ok();
    if let Some(s) = &reply_port {
        let _ = s.set_read_timeout(Some(Duration::from_millis(150)));
    }

    let target: SocketAddr = "255.255.255.255:7070".parse().map_err(|e| format!("{e}"))?;
    primary
        .send_to(&[0u8], target)
        .map_err(|e| format!("Broadcast send failed: {}", e))?;

    let start = Instant::now();
    let mut found: Vec<DiscoveredMiniserver> = Vec::new();
    let mut buf = [0u8; 2048];
    while start.elapsed() < timeout {
        for sock in std::iter::once(&primary).chain(reply_port.iter()) {
            if let Ok((n, from)) = sock.recv_from(&mut buf) {
                if n <= 1 {
                    continue; // our own probe echoed back
                }
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let entry = parse_discovery_reply(&text, &from.ip().to_string());
                if !found.iter().any(|f| f.raw == entry.raw) {
                    found.push(entry);
                }
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reply() {
        let r = parse_discovery_reply("LoxLIVE: My House 192.168.1.77:80 504F94A0ECAD 13.1.11.6", "1.2.3.4");
        assert_eq!(r.name, "My House");
        assert_eq!(r.address, "192.168.1.77:80");
        assert_eq!(r.serial, "504F94A0ECAD");
        let r = parse_discovery_reply("garbage", "10.0.0.5");
        assert_eq!(r.address, "10.0.0.5");
    }
}
