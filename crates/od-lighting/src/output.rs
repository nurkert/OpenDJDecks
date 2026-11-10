//! DMX outputs. Each turns a 512-channel universe into the bytes of one
//! protocol: Art-Net (ArtDmx over UDP 6454), sACN / E1.31 (UDP 5568) and the
//! Enttec DMX USB Pro serial protocol (label 6). Packet builders are pure so
//! they can be checked byte for byte; senders wrap them.

use std::io::Write;
use std::net::{SocketAddr, UdpSocket};

/// One DMX universe: 512 channel values (channel 1 = index 0).
pub type Universe = [u8; 512];

/// ArtDmx packet (Art-Net 4) for `universe` (15-bit port address).
pub fn artnet_packet(universe: u16, sequence: u8, data: &Universe) -> Vec<u8> {
    let mut p = Vec::with_capacity(18 + 512);
    p.extend_from_slice(b"Art-Net\0");
    p.extend_from_slice(&0x5000u16.to_le_bytes()); // OpDmx
    p.extend_from_slice(&14u16.to_be_bytes()); // protocol version
    p.push(sequence);
    p.push(0); // physical
    p.extend_from_slice(&(universe & 0x7FFF).to_le_bytes()); // SubUni, Net
    p.extend_from_slice(&512u16.to_be_bytes());
    p.extend_from_slice(data);
    p
}

/// sACN (ANSI E1.31-2018) data packet.
pub fn sacn_packet(
    universe: u16,
    sequence: u8,
    cid: &[u8; 16],
    source_name: &str,
    priority: u8,
    data: &Universe,
) -> Vec<u8> {
    let mut p = Vec::with_capacity(638);
    // Root layer.
    p.extend_from_slice(&0x0010u16.to_be_bytes()); // preamble size
    p.extend_from_slice(&0x0000u16.to_be_bytes()); // postamble size
    p.extend_from_slice(b"ASC-E1.17\0\0\0");
    p.extend_from_slice(&(0x7000u16 | (638 - 16)).to_be_bytes()); // flags + length
    p.extend_from_slice(&0x0000_0004u32.to_be_bytes()); // VECTOR_ROOT_E131_DATA
    p.extend_from_slice(cid);
    // Framing layer.
    p.extend_from_slice(&(0x7000u16 | (638 - 38)).to_be_bytes());
    p.extend_from_slice(&0x0000_0002u32.to_be_bytes()); // VECTOR_E131_DATA_PACKET
    let mut name = [0u8; 64];
    let bytes = source_name.as_bytes();
    name[..bytes.len().min(63)].copy_from_slice(&bytes[..bytes.len().min(63)]);
    p.extend_from_slice(&name);
    p.push(priority);
    p.extend_from_slice(&0u16.to_be_bytes()); // sync address
    p.push(sequence);
    p.push(0); // options
    p.extend_from_slice(&universe.to_be_bytes());
    // DMP layer.
    p.extend_from_slice(&(0x7000u16 | (638 - 115)).to_be_bytes());
    p.push(0x02); // VECTOR_DMP_SET_PROPERTY
    p.push(0xA1); // address type & data type
    p.extend_from_slice(&0u16.to_be_bytes()); // first property address
    p.extend_from_slice(&1u16.to_be_bytes()); // address increment
    p.extend_from_slice(&513u16.to_be_bytes()); // property count
    p.push(0); // DMX start code
    p.extend_from_slice(data);
    p
}

/// sACN multicast group of a universe (239.255.hi.lo).
pub fn sacn_multicast(universe: u16) -> std::net::Ipv4Addr {
    std::net::Ipv4Addr::new(239, 255, (universe >> 8) as u8, universe as u8)
}

/// Enttec DMX USB Pro "Output Only Send DMX Packet" (label 6) message.
pub fn enttec_pro_packet(data: &Universe) -> Vec<u8> {
    let len = (data.len() + 1) as u16;
    let mut p = Vec::with_capacity(5 + len as usize + 1);
    p.push(0x7E);
    p.push(6);
    p.extend_from_slice(&len.to_le_bytes());
    p.push(0); // start code
    p.extend_from_slice(data);
    p.push(0xE7);
    p
}

/// Where a universe goes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum OutputKind {
    /// Art-Net to an address (unicast or the broadcast address).
    ArtNet { target: SocketAddr, universe: u16 },
    /// sACN multicast for the universe.
    Sacn { universe: u16 },
    /// Enttec DMX USB Pro (or compatible) serial device path.
    EnttecPro { device: String },
}

/// A running output.
pub struct Output {
    kind: OutputKind,
    socket: Option<UdpSocket>,
    serial: Option<std::fs::File>,
    sequence: u8,
    cid: [u8; 16],
}

impl Output {
    pub fn open(kind: OutputKind) -> std::io::Result<Self> {
        let (socket, serial) = match &kind {
            OutputKind::ArtNet { .. } => {
                let s = UdpSocket::bind("0.0.0.0:0")?;
                s.set_broadcast(true)?;
                (Some(s), None)
            }
            OutputKind::Sacn { .. } => (Some(UdpSocket::bind("0.0.0.0:0")?), None),
            OutputKind::EnttecPro { device } => (
                None,
                Some(std::fs::OpenOptions::new().write(true).open(device)?),
            ),
        };
        // A stable component id per process (sACN requires one).
        let mut cid = *b"OpenDJDecks-DMX!";
        cid[15] = (std::process::id() & 0xFF) as u8;
        Ok(Self {
            kind,
            socket,
            serial,
            sequence: 0,
            cid,
        })
    }

    /// Sends one frame of `data`.
    pub fn send(&mut self, data: &Universe) -> std::io::Result<()> {
        self.sequence = self.sequence.wrapping_add(1).max(1);
        match &self.kind {
            OutputKind::ArtNet { target, universe } => {
                let p = artnet_packet(*universe, self.sequence, data);
                self.socket
                    .as_ref()
                    .expect("socket")
                    .send_to(&p, target)
                    .map(|_| ())
            }
            OutputKind::Sacn { universe } => {
                let p = sacn_packet(
                    *universe,
                    self.sequence,
                    &self.cid,
                    "OpenDJDecks",
                    100,
                    data,
                );
                let addr = SocketAddr::from((sacn_multicast(*universe), 5568));
                self.socket
                    .as_ref()
                    .expect("socket")
                    .send_to(&p, addr)
                    .map(|_| ())
            }
            OutputKind::EnttecPro { .. } => self
                .serial
                .as_mut()
                .expect("serial")
                .write_all(&enttec_pro_packet(data)),
        }
    }
}

/// Serial devices that may be a USB-DMX interface, for the port picker.
///
/// macOS lists `/dev/cu.usbserial*` and `/dev/cu.usbmodem*` (the call-out
/// devices, which open without waiting for a carrier), Linux `/dev/ttyUSB*`,
/// `/dev/ttyACM*` and the stable names in `/dev/serial/by-id`. On Windows
/// COM1 to COM64 are tried: a port that opens, or is busy, is listed. The
/// directory can be set with `OPENDJDECKS_SERIAL_DIR` (tests).
pub fn serial_ports() -> Vec<String> {
    if let Some(dir) = od_core::env::var_os("SERIAL_DIR") {
        return serial_ports_in(std::path::Path::new(&dir));
    }
    #[cfg(windows)]
    {
        (1..=64)
            .map(|n| format!("COM{n}"))
            .filter(|name| {
                match std::fs::OpenOptions::new()
                    .write(true)
                    .open(format!(r"\\.\{name}"))
                {
                    Ok(_) => true,
                    Err(e) => e.kind() == std::io::ErrorKind::PermissionDenied,
                }
            })
            .collect()
    }
    #[cfg(not(windows))]
    {
        let mut ports = serial_ports_in(std::path::Path::new("/dev"));
        let by_id = std::path::Path::new("/dev/serial/by-id");
        if by_id.is_dir() {
            ports.extend(
                std::fs::read_dir(by_id)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|e| e.path().display().to_string()),
            );
        }
        ports
    }
}

/// The serial devices in `dir` that look like USB adapters (see
/// [`serial_ports`]), sorted, as full paths.
pub fn serial_ports_in(dir: &std::path::Path) -> Vec<String> {
    const PREFIXES: [&str; 4] = ["cu.usbserial", "cu.usbmodem", "ttyUSB", "ttyACM"];
    let mut ports: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            PREFIXES
                .iter()
                .any(|p| name.starts_with(p))
                .then(|| dir.join(&name).display().to_string())
        })
        .collect();
    ports.sort();
    ports
}

/// Anything a universe can be sent through: a real [`Output`] or, in tests
/// and for the fault hook, a stand-in.
pub trait Link: Send {
    fn send(&mut self, data: &Universe) -> std::io::Result<()>;
}

impl Link for Output {
    fn send(&mut self, data: &Universe) -> std::io::Result<()> {
        Output::send(self, data)
    }
}

/// Opens the link for an output kind (injectable for tests).
pub type Opener = Box<dyn FnMut(&OutputKind) -> std::io::Result<Box<dyn Link>> + Send>;

/// Opens real outputs. When `OPENDJDECKS_DMX_FAULT` names a file, every
/// link fails (on open and on send) while that file exists: a fault hook
/// to show and test a lost interface without pulling a cable.
pub fn default_opener() -> Opener {
    let fault = od_core::env::var_os("DMX_FAULT").map(std::path::PathBuf::from);
    Box::new(move |kind: &OutputKind| -> std::io::Result<Box<dyn Link>> {
        let out = Box::new(Output::open(kind.clone())?);
        match &fault {
            Some(path) => {
                if path.exists() {
                    return Err(fault_error());
                }
                Ok(Box::new(FaultHook {
                    inner: out,
                    path: path.clone(),
                }))
            }
            None => Ok(out),
        }
    })
}

fn fault_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "interface unplugged (fault hook)",
    )
}

struct FaultHook {
    inner: Box<dyn Link>,
    path: std::path::PathBuf,
}

impl Link for FaultHook {
    fn send(&mut self, data: &Universe) -> std::io::Result<()> {
        if self.path.exists() {
            return Err(fault_error());
        }
        self.inner.send(data)
    }
}

/// A link for tests: records frames, and fails while `unplugged` is set
/// (opening fails too, like a missing USB device).
#[derive(Clone, Default)]
pub struct TestLink {
    pub unplugged: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub frames: std::sync::Arc<std::sync::Mutex<Vec<Universe>>>,
}

impl TestLink {
    pub fn set_unplugged(&self, on: bool) {
        self.unplugged
            .store(on, std::sync::atomic::Ordering::SeqCst);
    }

    /// Last frame received.
    pub fn last(&self) -> Option<Universe> {
        self.frames.lock().ok().and_then(|f| f.last().copied())
    }

    pub fn count(&self) -> usize {
        self.frames.lock().map(|f| f.len()).unwrap_or(0)
    }

    /// An opener that hands out this link.
    pub fn opener(&self) -> Opener {
        let link = self.clone();
        Box::new(move |_: &OutputKind| -> std::io::Result<Box<dyn Link>> {
            if link.unplugged.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(fault_error());
            }
            Ok(Box::new(link.clone()))
        })
    }
}

impl Link for TestLink {
    fn send(&mut self, data: &Universe) -> std::io::Result<()> {
        if self.unplugged.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(fault_error());
        }
        if let Ok(mut f) = self.frames.lock() {
            f.push(*data);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn universe() -> Universe {
        let mut u = [0u8; 512];
        u[0] = 255;
        u[511] = 7;
        u
    }

    #[test]
    fn artnet_layout() {
        let p = artnet_packet(0x0123, 9, &universe());
        assert_eq!(p.len(), 530);
        assert_eq!(&p[..8], b"Art-Net\0");
        assert_eq!(&p[8..10], &[0x00, 0x50]); // OpDmx, little endian
        assert_eq!(&p[10..12], &[0, 14]);
        assert_eq!(p[12], 9);
        assert_eq!(&p[14..16], &[0x23, 0x01]);
        assert_eq!(&p[16..18], &[2, 0]); // 512, big endian
        assert_eq!((p[18], p[529]), (255, 7));
    }

    #[test]
    fn sacn_layout() {
        let cid = [1u8; 16];
        let p = sacn_packet(1, 3, &cid, "OpenDJDecks", 100, &universe());
        assert_eq!(p.len(), 638);
        assert_eq!(&p[4..16], b"ASC-E1.17\0\0\0");
        assert_eq!(u16::from_be_bytes([p[16], p[17]]) & 0x0FFF, 622);
        assert_eq!(&p[22..38], &cid);
        assert_eq!(u16::from_be_bytes([p[38], p[39]]) & 0x0FFF, 600);
        assert_eq!(&p[44..55], b"OpenDJDecks");
        assert_eq!(p[108], 100); // priority
        assert_eq!(p[111], 3); // sequence
        assert_eq!(u16::from_be_bytes([p[113], p[114]]), 1); // universe
        assert_eq!(u16::from_be_bytes([p[115], p[116]]) & 0x0FFF, 523);
        assert_eq!(u16::from_be_bytes([p[123], p[124]]), 513);
        assert_eq!((p[125], p[126], p[637]), (0, 255, 7));
        assert_eq!(sacn_multicast(1), std::net::Ipv4Addr::new(239, 255, 0, 1));
    }

    #[test]
    fn enttec_layout() {
        let p = enttec_pro_packet(&universe());
        assert_eq!(&p[..4], &[0x7E, 6, 0x01, 0x02]); // 513 bytes
        assert_eq!((p[4], p[5], p[516], p[517]), (0, 255, 7, 0xE7));
    }

    #[test]
    fn serial_port_picker_lists_usb_adapters() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "cu.usbserial-DMX1",
            "cu.usbmodem2101",
            "tty.usbserial-DMX1",
            "ttyUSB0",
            "ttyACM1",
            "ttyS0",
            "null",
        ] {
            std::fs::write(dir.path().join(name), b"").unwrap();
        }
        let ports = serial_ports_in(dir.path());
        let names: Vec<String> = ports
            .iter()
            .map(|p| p.rsplit(['/', '\\']).next().unwrap().to_string())
            .collect();
        assert_eq!(
            names,
            ["cu.usbmodem2101", "cu.usbserial-DMX1", "ttyACM1", "ttyUSB0"]
        );
        assert!(serial_ports_in(&dir.path().join("missing")).is_empty());
    }

    #[test]
    fn artnet_reaches_a_listener() {
        let rx = UdpSocket::bind("127.0.0.1:0").unwrap();
        rx.set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut out = Output::open(OutputKind::ArtNet {
            target: rx.local_addr().unwrap(),
            universe: 0,
        })
        .unwrap();
        out.send(&universe()).unwrap();
        let mut buf = [0u8; 600];
        let n = rx.recv(&mut buf).unwrap();
        assert_eq!((n, buf[18]), (530, 255));
    }
}
