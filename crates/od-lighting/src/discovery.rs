//! Finding Art-Net nodes on the local network.
//!
//! Art-Net is a public protocol for DMX over UDP port 6454. A controller
//! asks "who is there?" with an ArtPoll packet sent to the broadcast
//! address; every node answers with an ArtPollReply that carries its IP
//! address, a short and a long name and the universes of its output ports.
//! Replies are addressed to port 6454 of the controller, so [`discover`]
//! listens there when it can, and on the port it sent from otherwise (which
//! many nodes answer as well).
//!
//! The packet builders are pure so they can be checked byte for byte, and
//! [`FakeNode`] answers polls on the loopback address, so discovery is
//! tested without a network or a real node.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// The Art-Net UDP port.
pub const PORT: u16 = 6454;
const ID: &[u8; 8] = b"Art-Net\0";
const OP_POLL: u16 = 0x2000;
const OP_POLL_REPLY: u16 = 0x2100;
/// Length of an ArtPollReply as built here (shorter replies from older
/// nodes are read as far as they go).
pub const REPLY_LEN: usize = 239;

/// An ArtPoll: who is there?
pub fn artpoll_packet() -> Vec<u8> {
    let mut p = Vec::with_capacity(14);
    p.extend_from_slice(ID);
    p.extend_from_slice(&OP_POLL.to_le_bytes());
    p.extend_from_slice(&14u16.to_be_bytes()); // protocol version
    p.push(0); // flags: reply once, no diagnostics
    p.push(0); // diagnostics priority
    p
}

/// True for an ArtPoll packet.
pub fn is_poll(buf: &[u8]) -> bool {
    buf.len() >= 12 && &buf[..8] == ID && u16::from_le_bytes([buf[8], buf[9]]) == OP_POLL
}

/// A node that answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtNode {
    /// The address the node gives for itself.
    pub ip: Ipv4Addr,
    pub short_name: String,
    pub long_name: String,
    /// Port-Address (15 bit) of each output port.
    pub universes: Vec<u16>,
    /// Where the reply came from.
    pub from: SocketAddr,
}

impl ArtNode {
    /// Where to send DMX for this node.
    pub fn target(&self) -> SocketAddr {
        SocketAddr::from((self.ip, PORT))
    }

    /// "Stage left · 192.0.2.10 · U 1, 2"
    pub fn label(&self) -> String {
        let name = if self.short_name.is_empty() {
            self.long_name.as_str()
        } else {
            self.short_name.as_str()
        };
        let universes: Vec<String> = self.universes.iter().map(|u| (u + 1).to_string()).collect();
        if universes.is_empty() {
            format!("{name} · {}", self.ip)
        } else {
            format!("{name} · {} · U {}", self.ip, universes.join(", "))
        }
    }
}

fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

/// Reads an ArtPollReply; anything else is `None`.
pub fn parse_poll_reply(buf: &[u8], from: SocketAddr) -> Option<ArtNode> {
    if buf.len() < 108 || &buf[..8] != ID || u16::from_le_bytes([buf[8], buf[9]]) != OP_POLL_REPLY {
        return None;
    }
    let ip = Ipv4Addr::new(buf[10], buf[11], buf[12], buf[13]);
    let short_name = text(&buf[26..44]);
    let long_name = text(&buf[44..108]);
    let mut universes = Vec::new();
    if buf.len() >= 194 {
        let net = (buf[18] & 0x7F) as u16;
        let sub = (buf[19] & 0x0F) as u16;
        let ports = (u16::from_be_bytes([buf[172], buf[173]]) as usize).min(4);
        for i in 0..ports {
            // Only ports that can output DMX.
            if buf[174 + i] & 0x80 != 0 {
                universes.push((net << 8) | (sub << 4) | (buf[190 + i] & 0x0F) as u16);
            }
        }
    }
    Some(ArtNode {
        ip,
        short_name,
        long_name,
        universes,
        from,
    })
}

/// An ArtPollReply for a node at `ip` with output ports on `universes`
/// (Port-Addresses; up to four, sharing net and sub-net). Used by the fake
/// node in tests and the documentation hook.
pub fn artpoll_reply_packet(ip: Ipv4Addr, short: &str, long: &str, universes: &[u16]) -> Vec<u8> {
    let mut p = vec![0u8; REPLY_LEN];
    p[..8].copy_from_slice(ID);
    p[8..10].copy_from_slice(&OP_POLL_REPLY.to_le_bytes());
    p[10..14].copy_from_slice(&ip.octets());
    p[14..16].copy_from_slice(&PORT.to_le_bytes());
    let first = universes.first().copied().unwrap_or(0);
    p[18] = ((first >> 8) & 0x7F) as u8;
    p[19] = ((first >> 4) & 0x0F) as u8;
    let s = short.as_bytes();
    p[26..26 + s.len().min(17)].copy_from_slice(&s[..s.len().min(17)]);
    let l = long.as_bytes();
    p[44..44 + l.len().min(63)].copy_from_slice(&l[..l.len().min(63)]);
    let ports = universes.len().min(4);
    p[172..174].copy_from_slice(&(ports as u16).to_be_bytes());
    for (i, u) in universes.iter().take(4).enumerate() {
        p[174 + i] = 0x80; // can output DMX
        p[182 + i] = 0x80; // data is being output
        p[190 + i] = (u & 0x0F) as u8;
    }
    p
}

/// Sends an ArtPoll from `socket` to each target and collects the replies
/// for `wait`. Nodes are listed once each, by address and name.
pub fn discover_with(
    socket: &UdpSocket,
    targets: &[SocketAddr],
    wait: Duration,
) -> std::io::Result<Vec<ArtNode>> {
    let poll = artpoll_packet();
    let mut sent = false;
    let mut last_err = None;
    for t in targets {
        match socket.send_to(&poll, t) {
            Ok(_) => sent = true,
            Err(e) => last_err = Some(e),
        }
    }
    if !sent && let Some(e) = last_err {
        return Err(e);
    }
    let deadline = Instant::now() + wait;
    let mut nodes: Vec<ArtNode> = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        socket.set_read_timeout(Some(left.max(Duration::from_millis(1))))?;
        match socket.recv_from(&mut buf) {
            Ok((n, from)) => {
                if let Some(node) = parse_poll_reply(&buf[..n], from)
                    && !nodes
                        .iter()
                        .any(|x| x.ip == node.ip && x.short_name == node.short_name)
                {
                    nodes.push(node);
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {} // Windows: ICMP
            Err(e) => return Err(e),
        }
    }
    nodes.sort_by(|a, b| a.ip.cmp(&b.ip).then(a.short_name.cmp(&b.short_name)));
    Ok(nodes)
}

/// Asks the local network (the broadcast address, or `targets` when given)
/// which Art-Net nodes are there, waiting `wait` for answers.
pub fn discover(targets: &[SocketAddr], wait: Duration) -> std::io::Result<Vec<ArtNode>> {
    // Replies go to port 6454; another lighting program may hold it.
    let socket = UdpSocket::bind(("0.0.0.0", PORT)).or_else(|_| UdpSocket::bind("0.0.0.0:0"))?;
    socket.set_broadcast(true)?;
    let broadcast = [SocketAddr::from((Ipv4Addr::BROADCAST, PORT))];
    let targets = if targets.is_empty() {
        &broadcast[..]
    } else {
        targets
    };
    discover_with(&socket, targets, wait)
}

/// A stand-in node on the loopback address: answers every ArtPoll with an
/// ArtPollReply naming `ip` (use a documentation address such as
/// 192.0.2.10). Stops when dropped.
pub struct FakeNode {
    pub addr: SocketAddr,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl FakeNode {
    pub fn start(
        ip: Ipv4Addr,
        short: &str,
        long: &str,
        universes: &[u16],
    ) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.set_read_timeout(Some(Duration::from_millis(50)))?;
        let addr = socket.local_addr()?;
        let reply = artpoll_reply_packet(ip, short, long, universes);
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::Builder::new()
            .name("artnet-fake-node".into())
            .spawn(move || {
                let mut buf = [0u8; 1024];
                while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                    if let Ok((n, from)) = socket.recv_from(&mut buf)
                        && is_poll(&buf[..n])
                    {
                        let _ = socket.send_to(&reply, from);
                    }
                }
            })?;
        Ok(Self {
            addr,
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for FakeNode {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_and_reply_layout() {
        let p = artpoll_packet();
        assert_eq!(p.len(), 14);
        assert_eq!(&p[..8], b"Art-Net\0");
        assert_eq!(&p[8..10], &[0x00, 0x20]); // OpPoll, little endian
        assert_eq!(&p[10..12], &[0, 14]);
        assert!(is_poll(&p));

        let ip = Ipv4Addr::new(192, 0, 2, 10);
        let r = artpoll_reply_packet(ip, "Stage left", "Two-port node", &[0x12, 0x13]);
        assert_eq!(r.len(), REPLY_LEN);
        assert_eq!(&r[8..10], &[0x00, 0x21]);
        assert_eq!(&r[10..14], &[192, 0, 2, 10]);
        assert_eq!((r[18], r[19]), (0, 1), "net 0, sub-net 1");
        assert_eq!(u16::from_be_bytes([r[172], r[173]]), 2);
        let from: SocketAddr = "127.0.0.1:6454".parse().unwrap();
        let node = parse_poll_reply(&r, from).unwrap();
        assert_eq!(node.ip, ip);
        assert_eq!(node.short_name, "Stage left");
        assert_eq!(node.long_name, "Two-port node");
        assert_eq!(node.universes, vec![0x12, 0x13]);
        assert_eq!(node.target(), "192.0.2.10:6454".parse().unwrap());
        assert_eq!(node.label(), "Stage left · 192.0.2.10 · U 19, 20");
        assert!(parse_poll_reply(&p, from).is_none(), "a poll is no reply");
        assert!(parse_poll_reply(&r[..60], from).is_none(), "too short");
    }

    /// Discovery without a network: a fake node on the loopback address
    /// answers the poll.
    #[test]
    fn discovers_a_fake_node_on_loopback() {
        let a =
            FakeNode::start(Ipv4Addr::new(192, 0, 2, 10), "Stage left", "Node A", &[0]).unwrap();
        let b = FakeNode::start(Ipv4Addr::new(192, 0, 2, 11), "Truss", "Node B", &[1, 2]).unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let nodes = discover_with(&socket, &[a.addr, b.addr], Duration::from_millis(1500)).unwrap();
        assert_eq!(nodes.len(), 2, "{nodes:?}");
        assert_eq!(nodes[0].short_name, "Stage left");
        assert_eq!(nodes[0].universes, vec![0]);
        assert_eq!(nodes[1].ip, Ipv4Addr::new(192, 0, 2, 11));
        assert_eq!(nodes[1].universes, vec![1, 2]);
        assert_eq!(nodes[1].from, b.addr, "the reply came from the fake");
        // Nobody there: an empty list after the wait, not an error.
        drop((a, b));
        let quiet = UdpSocket::bind("127.0.0.1:0").unwrap();
        let none = discover_with(
            &socket,
            &[quiet.local_addr().unwrap()],
            Duration::from_millis(200),
        )
        .unwrap();
        assert!(none.is_empty());
    }
}
