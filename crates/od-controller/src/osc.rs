//! OSC for the lights (roadmap [34]): a small address space that mirrors
//! the lighting actions of the mapping files, received over UDP, and the
//! music of the decks sent to another instance on the local network.
//!
//! Address space (all under `/opendjdecks`; numbers count from 1 as on
//! the screen; a missing "on" argument means on):
//!
//! | Address | Arguments | Does |
//! |---|---|---|
//! | `/opendjdecks/light/pad` | pad 1–8, [on] | plays a pad of the current page (held while on, for flash pads) |
//! | `/opendjdecks/light/page` | page 1–4 | shows a page of pads |
//! | `/opendjdecks/light/page/next`, `…/page/prev` | | next or previous page |
//! | `/opendjdecks/light/group` | group 1–, level 0–1 | a group master |
//! | `/opendjdecks/light/master` | level 0–1 | the master dimmer |
//! | `/opendjdecks/light/blackout` | [on] | blackout on or off (toggles without an argument) |
//! | `/opendjdecks/light/flash`, `…/strobe`, `…/fog` | [on] | the hold keys, on while on |
//! | `/opendjdecks/light/look` | look 1–6 | an EASY look |
//! | `/opendjdecks/light/mode` | `auto`, `easy` or `pro` | the mode |
//! | `/opendjdecks/light/cue/go`, `…/cue/back` | | the cue stack |
//! | `/opendjdecks/light/release`, `…/hold`, `…/tap` | | RELEASE, HOLD, TAP |
//! | `/opendjdecks/music` | bpm f, beat d, playing i, energy f, phrase s, hue f, remaining f | the music another instance follows |
//!
//! Numbers may come as int32, float32 or float64. Bundles are opened (their
//! time tag is ignored: everything acts at once). Anything else is ignored.

use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use od_core::PhraseKind;

use crate::action::LightingAction;

/// Prefix of the lighting addresses.
pub const LIGHT: &str = "/opendjdecks/light";
/// Address of the music message.
pub const MUSIC: &str = "/opendjdecks/music";

#[derive(Clone, Debug, PartialEq)]
pub enum OscArg {
    Int(i32),
    Float(f32),
    Double(f64),
    Str(String),
    Bool(bool),
}

impl OscArg {
    pub fn number(&self) -> Option<f64> {
        match self {
            OscArg::Int(i) => Some(*i as f64),
            OscArg::Float(f) => Some(*f as f64),
            OscArg::Double(d) => Some(*d),
            OscArg::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            OscArg::Str(_) => None,
        }
        .filter(|v| v.is_finite())
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            OscArg::Str(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OscMessage {
    pub addr: String,
    pub args: Vec<OscArg>,
}

impl OscMessage {
    pub fn new(addr: &str, args: Vec<OscArg>) -> Self {
        Self {
            addr: addr.to_string(),
            args,
        }
    }

    fn num(&self, i: usize) -> Option<f64> {
        self.args.get(i).and_then(OscArg::number)
    }

    /// The "on" argument: missing means on.
    fn on(&self, i: usize) -> bool {
        self.num(i).is_none_or(|v| v > 0.5)
    }
}

fn pad4(n: usize) -> usize {
    (n + 3) & !3
}

fn read_str(buf: &[u8], pos: &mut usize) -> Option<String> {
    let rest = buf.get(*pos..)?;
    let end = rest.iter().position(|b| *b == 0)?;
    let s = std::str::from_utf8(&rest[..end]).ok()?.to_string();
    *pos += pad4(end + 1);
    (*pos <= buf.len()).then_some(s)
}

fn read4(buf: &[u8], pos: &mut usize) -> Option<[u8; 4]> {
    let b: [u8; 4] = buf.get(*pos..*pos + 4)?.try_into().ok()?;
    *pos += 4;
    Some(b)
}

/// Decodes a packet: one message, or the messages of a bundle (nested
/// bundles too). `None` if it is not valid OSC.
pub fn decode(buf: &[u8]) -> Option<Vec<OscMessage>> {
    let mut out = Vec::new();
    decode_into(buf, &mut out, 0)?;
    Some(out)
}

fn decode_into(buf: &[u8], out: &mut Vec<OscMessage>, depth: u8) -> Option<()> {
    if depth > 8 {
        return None;
    }
    if buf.starts_with(b"#bundle\0") {
        let mut pos = 16; // "#bundle\0" and the time tag
        if buf.len() < pos {
            return None;
        }
        while pos < buf.len() {
            let len = i32::from_be_bytes(read4(buf, &mut pos)?);
            let len = usize::try_from(len).ok()?;
            let part = buf.get(pos..pos + len)?;
            decode_into(part, out, depth + 1)?;
            pos += len;
        }
        return Some(());
    }
    let mut pos = 0;
    let addr = read_str(buf, &mut pos)?;
    if !addr.starts_with('/') {
        return None;
    }
    let tags = if pos < buf.len() {
        read_str(buf, &mut pos)?
    } else {
        ",".into()
    };
    let tags = tags.strip_prefix(',')?;
    let mut args = Vec::new();
    for t in tags.chars() {
        args.push(match t {
            'i' => OscArg::Int(i32::from_be_bytes(read4(buf, &mut pos)?)),
            'f' => OscArg::Float(f32::from_be_bytes(read4(buf, &mut pos)?)),
            'd' => {
                let hi = read4(buf, &mut pos)?;
                let lo = read4(buf, &mut pos)?;
                let mut b = [0u8; 8];
                b[..4].copy_from_slice(&hi);
                b[4..].copy_from_slice(&lo);
                OscArg::Double(f64::from_be_bytes(b))
            }
            's' => OscArg::Str(read_str(buf, &mut pos)?),
            'T' => OscArg::Bool(true),
            'F' => OscArg::Bool(false),
            // Types this address space has no use for are not skippable
            // safely (blobs have a length, others vary): ignore the packet.
            _ => return None,
        });
    }
    out.push(OscMessage { addr, args });
    Some(())
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
    let n = pad4(s.len() + 1) - s.len();
    out.extend(std::iter::repeat_n(0u8, n));
}

/// Encodes one message.
pub fn encode(msg: &OscMessage) -> Vec<u8> {
    let mut out = Vec::new();
    write_str(&mut out, &msg.addr);
    let mut tags = String::from(",");
    for a in &msg.args {
        tags.push(match a {
            OscArg::Int(_) => 'i',
            OscArg::Float(_) => 'f',
            OscArg::Double(_) => 'd',
            OscArg::Str(_) => 's',
            OscArg::Bool(true) => 'T',
            OscArg::Bool(false) => 'F',
        });
    }
    write_str(&mut out, &tags);
    for a in &msg.args {
        match a {
            OscArg::Int(i) => out.extend_from_slice(&i.to_be_bytes()),
            OscArg::Float(f) => out.extend_from_slice(&f.to_be_bytes()),
            OscArg::Double(d) => out.extend_from_slice(&d.to_be_bytes()),
            OscArg::Str(s) => write_str(&mut out, s),
            OscArg::Bool(_) => {}
        }
    }
    out
}

/// The lighting action a message asks for, if any.
pub fn lighting_action(msg: &OscMessage) -> Option<LightingAction> {
    use LightingAction as L;
    let path = msg.addr.strip_prefix(LIGHT)?;
    let count = |i: usize, max: f64| {
        msg.num(i)
            .map(f64::round)
            .filter(|v| (1.0..=max).contains(v))
            .map(|v| v as u8 - 1)
    };
    let level = |i: usize| msg.num(i).map(|v| v.clamp(0.0, 1.0) as f32);
    Some(match path {
        "/pad" => L::Pad {
            pad: count(0, 8.0)?,
            pressed: msg.on(1),
        },
        "/page" => L::Page(count(0, 4.0)?),
        "/page/next" => L::PageNext,
        "/page/prev" => L::PagePrev,
        "/group" => L::GroupMaster {
            group: count(0, 255.0)?,
            level: level(1)?,
        },
        "/master" => L::Master(level(0)?),
        "/blackout" if msg.args.is_empty() => L::BlackoutToggle,
        "/blackout" => L::Blackout(msg.on(0)),
        "/flash" => L::Flash(msg.on(0)),
        "/strobe" => L::Strobe(msg.on(0)),
        "/fog" => L::Fog(msg.on(0)),
        "/look" => L::Look(count(0, 6.0)?),
        "/mode" => {
            let m = msg.args.first()?.text()?.to_ascii_lowercase();
            if !["auto", "easy", "pro"].contains(&m.as_str()) {
                return None;
            }
            L::Mode(m)
        }
        "/cue/go" => L::CueGo,
        "/cue/back" => L::CueBack,
        "/release" => L::Release,
        "/hold" => L::Hold,
        "/tap" => L::Tap,
        _ => return None,
    })
}

/// The music one instance sends another (what its show follows).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MusicMessage {
    pub bpm: f64,
    pub beat: f64,
    pub playing: bool,
    pub energy: f32,
    pub phrase: Option<PhraseKind>,
    pub hue: f32,
    /// Seconds left in the track (`None` = unknown).
    pub remaining: Option<f32>,
}

fn phrase_name(p: Option<PhraseKind>) -> &'static str {
    match p {
        None => "",
        Some(PhraseKind::Intro) => "intro",
        Some(PhraseKind::Verse) => "verse",
        Some(PhraseKind::Chorus) => "chorus",
        Some(PhraseKind::Bridge) => "bridge",
        Some(PhraseKind::Up) => "up",
        Some(PhraseKind::Down) => "down",
        Some(PhraseKind::Outro) => "outro",
    }
}

fn phrase_of(s: &str) -> Option<PhraseKind> {
    Some(match s {
        "intro" => PhraseKind::Intro,
        "verse" => PhraseKind::Verse,
        "chorus" => PhraseKind::Chorus,
        "bridge" => PhraseKind::Bridge,
        "up" => PhraseKind::Up,
        "down" => PhraseKind::Down,
        "outro" => PhraseKind::Outro,
        _ => return None,
    })
}

impl MusicMessage {
    pub fn to_osc(&self) -> OscMessage {
        OscMessage::new(
            MUSIC,
            vec![
                OscArg::Float(self.bpm as f32),
                OscArg::Double(self.beat),
                OscArg::Int(self.playing as i32),
                OscArg::Float(self.energy),
                OscArg::Str(phrase_name(self.phrase).into()),
                OscArg::Float(self.hue),
                OscArg::Float(self.remaining.unwrap_or(-1.0)),
            ],
        )
    }

    pub fn from_osc(msg: &OscMessage) -> Option<Self> {
        if msg.addr != MUSIC {
            return None;
        }
        let bpm = msg.num(0)?;
        let beat = msg.num(1)?;
        if !(20.0..=400.0).contains(&bpm) {
            return None;
        }
        Some(Self {
            bpm,
            beat,
            playing: msg.num(2).is_some_and(|v| v > 0.5),
            energy: msg.num(3).unwrap_or(0.5).clamp(0.0, 1.0) as f32,
            phrase: msg.args.get(4).and_then(OscArg::text).and_then(phrase_of),
            hue: msg.num(5).unwrap_or(0.6).clamp(0.0, 1.0) as f32,
            remaining: msg.num(6).filter(|r| *r >= 0.0).map(|r| r as f32),
        })
    }
}

/// What arrived over OSC.
#[derive(Clone, Debug, PartialEq)]
pub enum OscEvent {
    Light(LightingAction),
    Music(MusicMessage),
}

/// The meaning of a message, if this address space has one.
pub fn event(msg: &OscMessage) -> Option<OscEvent> {
    if let Some(m) = MusicMessage::from_osc(msg) {
        return Some(OscEvent::Music(m));
    }
    lighting_action(msg).map(OscEvent::Light)
}

/// Listens for OSC on a UDP port; events arrive in [`OscReceiver::events`].
/// Stops when dropped.
pub struct OscReceiver {
    events: Receiver<OscEvent>,
    stop: Arc<AtomicBool>,
    addr: SocketAddr,
    ignored: Arc<AtomicU64>,
    received: Arc<AtomicU64>,
}

impl OscReceiver {
    /// Binds `addr` (port 0: any free port) and starts listening.
    pub fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        let addr = socket.local_addr()?;
        let (tx, rx) = crossbeam_channel::unbounded();
        let stop = Arc::new(AtomicBool::new(false));
        let ignored = Arc::new(AtomicU64::new(0));
        let received = Arc::new(AtomicU64::new(0));
        let flags = (stop.clone(), ignored.clone(), received.clone());
        std::thread::Builder::new()
            .name("od-osc".into())
            .spawn(move || listen(socket, tx, flags))?;
        Ok(Self {
            events: rx,
            stop,
            addr,
            ignored,
            received,
        })
    }

    pub fn events(&self) -> &Receiver<OscEvent> {
        &self.events
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Messages understood so far.
    pub fn received(&self) -> u64 {
        self.received.load(Ordering::Relaxed)
    }

    /// Packets or messages that were not OSC or not ours.
    pub fn ignored(&self) -> u64 {
        self.ignored.load(Ordering::Relaxed)
    }
}

impl Drop for OscReceiver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn listen(
    socket: UdpSocket,
    tx: Sender<OscEvent>,
    (stop, ignored, received): (Arc<AtomicBool>, Arc<AtomicU64>, Arc<AtomicU64>),
) {
    let mut buf = vec![0u8; 8192];
    while !stop.load(Ordering::Relaxed) {
        let n = match socket.recv_from(&mut buf) {
            Ok((n, _)) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(e) => {
                tracing::debug!("osc: {e}");
                continue;
            }
        };
        let Some(msgs) = decode(&buf[..n]) else {
            ignored.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        for m in msgs {
            match event(&m) {
                Some(e) => {
                    received.fetch_add(1, Ordering::Relaxed);
                    if tx.send(e).is_err() {
                        return;
                    }
                }
                None => {
                    ignored.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}

/// Sends messages to one address over UDP.
pub struct OscSender {
    socket: UdpSocket,
    target: SocketAddr,
}

impl OscSender {
    pub fn new(target: SocketAddr) -> std::io::Result<Self> {
        let bind: SocketAddr = if target.is_ipv4() {
            "0.0.0.0:0".parse().expect("address")
        } else {
            "[::]:0".parse().expect("address")
        };
        let socket = UdpSocket::bind(bind)?;
        Ok(Self { socket, target })
    }

    pub fn target(&self) -> SocketAddr {
        self.target
    }

    pub fn send(&self, msg: &OscMessage) -> std::io::Result<()> {
        self.socket.send_to(&encode(msg), self.target).map(|_| ())
    }
}

/// Reads "host:port" (a port alone means this computer).
pub fn parse_target(text: &str) -> Result<SocketAddr, String> {
    use std::net::ToSocketAddrs;
    let t = text.trim();
    if t.is_empty() {
        return Err("no address".into());
    }
    if let Ok(port) = t.parse::<u16>() {
        return Ok(SocketAddr::from(([127, 0, 0, 1], port)));
    }
    t.to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| "no address".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_round_trip() {
        let m = OscMessage::new(
            "/opendjdecks/light/group",
            vec![OscArg::Int(2), OscArg::Float(0.25), OscArg::Str("x".into())],
        );
        let bytes = encode(&m);
        assert_eq!(bytes.len() % 4, 0);
        assert_eq!(decode(&bytes).unwrap(), vec![m]);
        let music = MusicMessage {
            bpm: 128.0,
            beat: 1234.5678,
            playing: true,
            energy: 0.75,
            phrase: Some(PhraseKind::Chorus),
            hue: 0.3,
            remaining: Some(42.0),
        };
        let back = MusicMessage::from_osc(&decode(&encode(&music.to_osc())).unwrap()[0]).unwrap();
        assert_eq!(back, music);
    }

    #[test]
    fn the_address_space_mirrors_the_lighting_actions() {
        use LightingAction as L;
        let a = |addr: &str, args: Vec<OscArg>| lighting_action(&OscMessage::new(addr, args));
        assert_eq!(
            a("/opendjdecks/light/pad", vec![OscArg::Int(3)]),
            Some(L::Pad { pad: 2, pressed: true })
        );
        assert_eq!(
            a("/opendjdecks/light/pad", vec![OscArg::Float(8.0), OscArg::Float(0.0)]),
            Some(L::Pad { pad: 7, pressed: false })
        );
        assert_eq!(a("/opendjdecks/light/pad", vec![OscArg::Int(9)]), None, "pads 1 to 8");
        assert_eq!(a("/opendjdecks/light/page", vec![OscArg::Int(4)]), Some(L::Page(3)));
        assert_eq!(a("/opendjdecks/light/page/next", vec![]), Some(L::PageNext));
        assert_eq!(
            a("/opendjdecks/light/group", vec![OscArg::Int(1), OscArg::Double(2.0)]),
            Some(L::GroupMaster { group: 0, level: 1.0 })
        );
        assert_eq!(a("/opendjdecks/light/master", vec![OscArg::Float(0.5)]), Some(L::Master(0.5)));
        assert_eq!(a("/opendjdecks/light/blackout", vec![]), Some(L::BlackoutToggle));
        assert_eq!(a("/opendjdecks/light/blackout", vec![OscArg::Int(0)]), Some(L::Blackout(false)));
        assert_eq!(a("/opendjdecks/light/strobe", vec![OscArg::Bool(true)]), Some(L::Strobe(true)));
        assert_eq!(a("/opendjdecks/light/flash", vec![]), Some(L::Flash(true)));
        assert_eq!(a("/opendjdecks/light/fog", vec![OscArg::Int(0)]), Some(L::Fog(false)));
        assert_eq!(a("/opendjdecks/light/look", vec![OscArg::Int(6)]), Some(L::Look(5)));
        assert_eq!(a("/opendjdecks/light/mode", vec![OscArg::Str("PRO".into())]), Some(L::Mode("pro".into())));
        assert_eq!(a("/opendjdecks/light/mode", vec![OscArg::Str("disco".into())]), None);
        assert_eq!(a("/opendjdecks/light/cue/go", vec![]), Some(L::CueGo));
        assert_eq!(a("/opendjdecks/light/tap", vec![]), Some(L::Tap));
        assert_eq!(a("/opendjdecks/deck/play", vec![]), None, "only the lights");
    }

    #[test]
    fn bundles_and_garbage() {
        let a = encode(&OscMessage::new("/opendjdecks/light/hold", vec![]));
        let b = encode(&OscMessage::new("/opendjdecks/light/release", vec![]));
        let mut bundle = b"#bundle\0".to_vec();
        bundle.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
        for m in [&a, &b] {
            bundle.extend_from_slice(&(m.len() as i32).to_be_bytes());
            bundle.extend_from_slice(m);
        }
        let msgs = decode(&bundle).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(lighting_action(&msgs[1]), Some(LightingAction::Release));
        for junk in [&b""[..], b"hello", b"/x\0\0,z\0\0", &[0xff; 40], b"#bundle\0\0\0\0\0\0\0\0\0\xff\xff\xff\xff"] {
            assert!(decode(junk).is_none(), "{junk:?}");
        }
    }

    /// A loopback sender: lighting actions and the music arrive as events;
    /// what is not ours is counted and dropped.
    #[test]
    fn a_loopback_sender_reaches_the_receiver() {
        let rx = OscReceiver::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let tx = OscSender::new(rx.local_addr()).unwrap();
        tx.send(&OscMessage::new("/opendjdecks/light/pad", vec![OscArg::Int(2)])).unwrap();
        tx.send(&OscMessage::new("/somebody/else", vec![])).unwrap();
        let music = MusicMessage {
            bpm: 174.0,
            beat: 64.0,
            playing: true,
            energy: 0.9,
            phrase: Some(PhraseKind::Down),
            hue: 0.1,
            remaining: None,
        };
        tx.send(&music.to_osc()).unwrap();
        let t = Duration::from_secs(3);
        assert_eq!(
            rx.events().recv_timeout(t).unwrap(),
            OscEvent::Light(LightingAction::Pad { pad: 1, pressed: true })
        );
        assert_eq!(rx.events().recv_timeout(t).unwrap(), OscEvent::Music(music));
        assert_eq!(rx.received(), 2);
        assert_eq!(rx.ignored(), 1);
        assert_eq!(parse_target("9300").unwrap().port(), 9300);
        assert!(parse_target("").is_err());
        assert_eq!(parse_target("127.0.0.1:7000").unwrap().port(), 7000);
    }
}
