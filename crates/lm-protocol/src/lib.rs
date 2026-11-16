//! Shared types between the audio engine, the Tauri shell, and (via serde)
//! the frontend. Serialization is camelCase to match the TypeScript mirror in
//! `src/lib/types.ts` — keep the two in sync.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;

/// Bus slots, VoiceMeeter Potato style: A buses feed hardware outputs, B buses
/// are virtual sources (microphones other apps can record). A profile holds
/// any subset; `Ord` is display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BusId {
    A1,
    A2,
    A3,
    A4,
    A5,
    B1,
    B2,
    B3,
}

impl BusId {
    pub const ALL: [BusId; 8] =
        [BusId::A1, BusId::A2, BusId::A3, BusId::A4, BusId::A5, BusId::B1, BusId::B2, BusId::B3];

    /// Lowercase node-name fragment: `lm.bus.a1`.
    pub fn node_base(&self) -> String {
        format!("lm.bus.{}", format!("{self:?}").to_lowercase())
    }

    /// A buses drive a hardware output; B buses are virtual sources.
    pub fn is_output(&self) -> bool {
        matches!(self, BusId::A1 | BusId::A2 | BusId::A3 | BusId::A4 | BusId::A5)
    }
}

/// Upper bound on strips of all kinds, to keep the rack usable.
pub const MAX_STRIPS: usize = 16;
/// Longest label a strip or bus may carry.
pub const LABEL_MAX: usize = 32;

/// Normalize a user-entered strip/bus label: trimmed, control characters
/// removed, at most [`LABEL_MAX`] characters. None if nothing is left.
pub fn clean_label(raw: &str) -> Option<String> {
    let s: String = raw.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(LABEL_MAX).collect();
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

pub type StripId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StripKind {
    /// Appears as an Audio/Sink other apps can play into.
    Virtual,
    /// Captures from a hardware (or any existing) source node.
    Hardware,
    /// Receives a VBAN network stream (VoiceMeeter, VBAN apps, another
    /// linuxmeeter). Not a device: nothing but our own receiver feeds it.
    Vban,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateParams {
    pub enabled: bool,
    pub threshold_db: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub hold_ms: f32,
}

impl Default for GateParams {
    fn default() -> Self {
        Self { enabled: false, threshold_db: -40.0, attack_ms: 20.0, release_ms: 100.0, hold_ms: 50.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompParams {
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
}

impl Default for CompParams {
    fn default() -> Self {
        Self { enabled: false, threshold_db: -18.0, ratio: 4.0, attack_ms: 20.0, release_ms: 100.0, makeup_db: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqBandKind {
    LowShelf,
    Peak,
    HighShelf,
}

impl EqBandKind {
    /// LSP para_equalizer `ft_N` filter-type value.
    pub fn lsp_filter_type(&self) -> f32 {
        match self {
            EqBandKind::Peak => 1.0,      // Bell
            EqBandKind::HighShelf => 3.0, // Hi-shelf
            EqBandKind::LowShelf => 5.0,  // Lo-shelf
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqBand {
    pub kind: EqBandKind,
    pub freq_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqParams {
    pub enabled: bool,
    pub bands: Vec<EqBand>,
}

impl EqParams {
    /// Bands 4..8 are reserved for the voice-color pad (voice-tuned corners),
    /// independent of the four user-facing EQ-panel bands.
    pub fn color_defaults() -> [EqBand; 4] {
        [
            EqBand { kind: EqBandKind::LowShelf, freq_hz: 300.0, gain_db: 0.0, q: 0.7 },
            EqBand { kind: EqBandKind::Peak, freq_hz: 400.0, gain_db: 0.0, q: 0.8 },
            EqBand { kind: EqBandKind::Peak, freq_hz: 3000.0, gain_db: 0.0, q: 0.8 },
            EqBand { kind: EqBandKind::HighShelf, freq_hz: 3500.0, gain_db: 0.0, q: 0.7 },
        ]
    }
}

impl EqParams {
    /// A bus EQ: all eight bands are user bands (no color pad on buses),
    /// spread across the spectrum. Mirrors `BUS_EQ_BANDS` in types.ts.
    pub fn bus_default() -> Self {
        let band = |kind, freq_hz, q| EqBand { kind, freq_hz, gain_db: 0.0, q };
        Self {
            enabled: false,
            bands: vec![
                band(EqBandKind::LowShelf, 60.0, 0.7),
                band(EqBandKind::Peak, 150.0, 1.0),
                band(EqBandKind::Peak, 400.0, 1.0),
                band(EqBandKind::Peak, 1000.0, 1.0),
                band(EqBandKind::Peak, 2500.0, 1.0),
                band(EqBandKind::Peak, 5000.0, 1.0),
                band(EqBandKind::Peak, 8000.0, 1.0),
                band(EqBandKind::HighShelf, 12000.0, 0.7),
            ],
        }
    }
}

impl Default for EqParams {
    fn default() -> Self {
        let mut bands = vec![
            EqBand { kind: EqBandKind::LowShelf, freq_hz: 100.0, gain_db: 0.0, q: 0.7 },
            EqBand { kind: EqBandKind::Peak, freq_hz: 400.0, gain_db: 0.0, q: 1.0 },
            EqBand { kind: EqBandKind::Peak, freq_hz: 2500.0, gain_db: 0.0, q: 1.0 },
            EqBand { kind: EqBandKind::HighShelf, freq_hz: 8000.0, gain_db: 0.0, q: 0.7 },
        ];
        bands.extend(Self::color_defaults());
        Self { enabled: false, bands }
    }
}

/// A strip's stereo image, applied after the EQ: VoiceMeeter's mono and
/// stereo-reverse buttons plus a balance control.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PanParams {
    /// Sum both channels to mono (wins over `reverse`, which is moot then).
    pub mono: bool,
    /// Swap left and right.
    pub reverse: bool,
    /// -1 = hard left, 0 = centre, 1 = hard right. Attenuates the opposite
    /// side only, so centre is unity and nothing is ever boosted.
    pub balance: f32,
}

impl Default for PanParams {
    fn default() -> Self {
        Self { mono: false, reverse: false, balance: 0.0 }
    }
}

impl PanParams {
    /// The 2x2 gain matrix the strip applies: `[[L<-L, L<-R], [R<-L, R<-R]]`.
    pub fn matrix(&self) -> [[f32; 2]; 2] {
        let mut m = if self.mono {
            [[0.5, 0.5], [0.5, 0.5]]
        } else if self.reverse {
            [[0.0, 1.0], [1.0, 0.0]]
        } else {
            [[1.0, 0.0], [0.0, 1.0]]
        };
        let b = if self.balance.is_finite() { self.balance.clamp(-1.0, 1.0) } else { 0.0 };
        let (gl, gr) = ((1.0 - b).min(1.0), (1.0 + b).min(1.0));
        for v in &mut m[0] {
            *v *= gl;
        }
        for v in &mut m[1] {
            *v *= gr;
        }
        m
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimiterParams {
    pub enabled: bool,
    pub threshold_db: f32,
    pub release_ms: f32,
}

impl Default for LimiterParams {
    fn default() -> Self {
        Self { enabled: true, threshold_db: -1.0, release_ms: 50.0 }
    }
}

// ---- VBAN -----------------------------------------------------------------

/// Standard VBAN UDP port.
pub const VBAN_DEFAULT_PORT: u16 = 6980;
/// The VBAN header carries a 16-byte stream name.
pub const VBAN_STREAM_NAME_MAX: usize = 16;
/// Sample rates offered for outgoing streams (all are VBAN-legal).
pub const VBAN_RATES: [u32; 3] = [44100, 48000, 96000];

/// Normalize a VBAN stream name: printable ASCII only, at most 16 bytes.
/// A leading `~` is dropped because PipeWire's match rules would read it as a
/// regex. Empty input becomes `Stream1`, VoiceMeeter's default name.
pub fn vban_stream_name(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .trim_start_matches('~')
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(VBAN_STREAM_NAME_MAX)
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        "Stream1".into()
    } else {
        cleaned.to_string()
    }
}

/// Incoming VBAN stream feeding a [`StripKind::Vban`] strip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VbanRecv {
    /// Only packets carrying this stream name are accepted (PipeWire ≥ 1.4;
    /// older versions take whatever arrives on the port).
    pub stream_name: String,
    /// Local UDP port to listen on.
    pub port: u16,
    /// Only accept packets from this sender. Empty = any sender.
    pub sender_ip: String,
    /// Jitter buffer target, in milliseconds.
    pub latency_ms: u32,
}

impl Default for VbanRecv {
    fn default() -> Self {
        Self { stream_name: "Stream1".into(), port: VBAN_DEFAULT_PORT, sender_ip: String::new(), latency_ms: 100 }
    }
}

impl VbanRecv {
    /// Normalize and check; returns the cleaned config or a user-facing error.
    pub fn validated(&self) -> Result<Self, String> {
        let sender_ip = self.sender_ip.trim().to_string();
        if !sender_ip.is_empty() && sender_ip.parse::<IpAddr>().is_err() {
            return Err(format!("\"{sender_ip}\" is not an IP address"));
        }
        if self.port == 0 {
            return Err("port must be 1-65535".into());
        }
        Ok(Self {
            stream_name: vban_stream_name(&self.stream_name),
            port: self.port,
            sender_ip,
            latency_ms: self.latency_ms.clamp(10, 2000),
        })
    }
}

/// Outgoing VBAN stream carrying a bus's post-limiter mix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VbanSend {
    pub enabled: bool,
    pub stream_name: String,
    /// Destination IP address (unicast or broadcast). PipeWire's sender does
    /// not resolve hostnames, so this must be a literal address.
    pub host: String,
    pub port: u16,
    pub rate: u32,
}

impl Default for VbanSend {
    fn default() -> Self {
        Self {
            enabled: false,
            stream_name: "Stream1".into(),
            host: "127.0.0.1".into(),
            port: VBAN_DEFAULT_PORT,
            rate: 48000,
        }
    }
}

impl VbanSend {
    /// Normalize and check; returns the cleaned config or a user-facing error.
    pub fn validated(&self) -> Result<Self, String> {
        let host = self.host.trim().to_string();
        if host.parse::<IpAddr>().is_err() {
            return Err(format!("\"{host}\" is not an IP address"));
        }
        if self.port == 0 {
            return Err("port must be 1-65535".into());
        }
        if !VBAN_RATES.contains(&self.rate) {
            return Err(format!("unsupported sample rate {}", self.rate));
        }
        Ok(Self {
            enabled: self.enabled,
            stream_name: vban_stream_name(&self.stream_name),
            host,
            port: self.port,
            rate: self.rate,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StripState {
    pub id: StripId,
    pub kind: StripKind,
    pub label: String,
    /// Stable PipeWire identity (`node.name`) of the hardware source, for Hardware strips.
    pub hw_key: Option<String>,
    pub online: bool,
    pub gain_db: f32,
    pub mute: bool,
    pub solo: bool,
    pub routes: BTreeMap<BusId, bool>,
    pub gate: GateParams,
    pub comp: CompParams,
    /// `default` so profiles written before the EQ existed still load.
    #[serde(default)]
    pub eq: EqParams,
    /// `default` so profiles written before panning existed still load.
    #[serde(default)]
    pub pan: PanParams,
    /// Receiver config; Some exactly for [`StripKind::Vban`] strips.
    #[serde(default)]
    pub vban: Option<VbanRecv>,
}

impl StripState {
    /// Base node name; virtual strips' sink node carries this name directly.
    pub fn node_base(&self) -> String {
        format!("lm.strip.{}", self.id)
    }

    /// Node that carries Props (volume/mute/filter params): the capture side.
    pub fn control_node(&self) -> String {
        match self.kind {
            StripKind::Virtual | StripKind::Vban => self.node_base(),
            StripKind::Hardware => format!("{}.cap", self.node_base()),
        }
    }

    /// Node whose output ports feed the routing matrix.
    pub fn out_node(&self) -> String {
        format!("{}.out", self.node_base())
    }

    /// The VBAN receiver's output stream, linked into this strip's input.
    pub fn vban_node(&self) -> String {
        format!("{}.vban", self.node_base())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusState {
    pub id: BusId,
    pub label: String,
    /// `node.name` of the hardware sink this bus feeds (A buses). None = virtual source (B buses).
    pub target_hw_key: Option<String>,
    pub online: bool,
    pub gain_db: f32,
    pub mute: bool,
    pub limiter: LimiterParams,
    /// Runs before the limiter. Profiles written before bus EQ existed get
    /// a flat, disabled one.
    #[serde(default = "EqParams::bus_default")]
    pub eq: EqParams,
    /// `default` so profiles written before VBAN existed still load.
    #[serde(default)]
    pub vban: VbanSend,
}

impl BusState {
    pub fn node_base(&self) -> String {
        self.id.node_base()
    }

    /// Mix-input node (also carries Props for gain/mute/limiter).
    pub fn in_node(&self) -> String {
        format!("{}.in", self.node_base())
    }

    /// Node whose output ports carry the post-limiter signal (meter tap point).
    pub fn tap_node(&self) -> String {
        if self.target_hw_key.is_some() {
            format!("{}.out", self.node_base())
        } else {
            self.node_base()
        }
    }

    /// The VBAN sender's input stream, fed from `tap_node` (post-limiter).
    pub fn vban_node(&self) -> String {
        format!("{}.vban", self.node_base())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub node_name: String,
    pub description: String,
    pub media_class: String,
    pub serial: u64,
    pub channels: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub strips: Vec<StripState>,
    pub buses: Vec<BusState>,
    pub devices: Vec<DeviceInfo>,
    pub profile_name: String,
    /// True while linuxmeeter's System strip is the system default output.
    pub take_default_output: bool,
}

/// One batched meter update, ~30/s. Channel layout: [peakL, peakR, rmsL, rmsR]
/// in dBFS; entries follow `AppState.strips` / `AppState.buses` order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeterFrame {
    pub seq: u32,
    pub strips: Vec<[f32; 4]>,
    pub buses: Vec<[f32; 4]>,
}

/// A strip or a bus, as sent by the frontend: `{"strip": 3}` or `{"bus": "A1"}`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Target {
    Strip { strip: StripId },
    Bus { bus: BusId },
}

/// Commands into the engine thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineCommand {
    SetGain { target: Target, db: f32 },
    SetMute { target: Target, mute: bool },
    SetSolo { strip: StripId, solo: bool },
    SetRoute { strip: StripId, bus: BusId, on: bool },
    SetGateParams { strip: StripId, params: GateParams },
    SetCompParams { strip: StripId, params: CompParams },
    SetEqParams { strip: StripId, params: EqParams },
    SetPanParams { strip: StripId, params: PanParams },
    SetLimiterParams { bus: BusId, params: LimiterParams },
    SetBusEqParams { bus: BusId, params: EqParams },
    /// Point a hardware strip at a different capture device (module reload).
    SetStripDevice { strip: StripId, hw_key: String },
    /// Point an A-bus at a different output device (module reload).
    SetBusTarget { bus: BusId, hw_key: String },
    /// Configure (or disable) a bus's outgoing VBAN stream.
    SetBusVban { bus: BusId, config: VbanSend },
    /// Add a strip fed by an incoming VBAN stream.
    AddVbanStrip { label: String, config: VbanRecv },
    /// Reconfigure a VBAN strip's receiver (label included).
    SetStripVban { strip: StripId, label: String, config: VbanRecv },
    /// Add an empty hardware (capturing the default input) or virtual strip.
    /// VBAN strips are added with [`EngineCommand::AddVbanStrip`].
    AddStrip { kind: StripKind, label: String },
    /// Remove a strip of any kind.
    RemoveStrip { strip: StripId },
    /// Add the next free A bus (`output: true`, hardware output) or B bus
    /// (virtual source).
    AddBus { output: bool },
    /// Remove a bus; every route into it goes with it.
    RemoveBus { bus: BusId },
    /// Rename a strip or bus.
    SetLabel { target: Target, label: String },
    /// Make the System strip the system-wide default output (VoiceMeeter
    /// style: all desktop audio flows through the mixer), or restore the
    /// previous hardware default.
    SetDefaultOutput { on: bool },
    /// Start/stop the 30 Hz meter drain. Every frame costs a script evaluation
    /// in the webview, so a mixer sitting in the tray stops paying for meters
    /// nobody can see. Audio is unaffected either way.
    SetMetersEnabled { on: bool },
    Shutdown,
}

/// Events out of the engine thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineEvent {
    /// Full snapshot; sent on startup and after any state-affecting change.
    State(AppState),
    Meters(MeterFrame),
    /// A rejected command, for the UI to surface (e.g. an invalid VBAN address).
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(id: StripId, kind: StripKind) -> StripState {
        StripState {
            id,
            kind,
            label: "Test".into(),
            hw_key: None,
            online: true,
            gain_db: 0.0,
            mute: false,
            solo: false,
            routes: BusId::ALL.iter().map(|b| (*b, false)).collect(),
            gate: GateParams::default(),
            comp: CompParams::default(),
            eq: EqParams::default(),
            pan: Default::default(),
            vban: (kind == StripKind::Vban).then(VbanRecv::default),
        }
    }

    fn bus(id: BusId, target: Option<&str>) -> BusState {
        BusState {
            id,
            label: "Test".into(),
            target_hw_key: target.map(String::from),
            online: true,
            gain_db: 0.0,
            mute: false,
            limiter: LimiterParams::default(),
            eq: EqParams::bus_default(),
            vban: VbanSend::default(),
        }
    }

    // ---- node naming: the identity profiles, routes, and taps all key on ----

    #[test]
    fn virtual_strip_controls_its_sink_node_directly() {
        let s = strip(3, StripKind::Virtual);
        assert_eq!(s.node_base(), "lm.strip.3");
        assert_eq!(s.control_node(), "lm.strip.3", "the sink node carries Props");
        assert_eq!(s.out_node(), "lm.strip.3.out");
    }

    /// A hardware strip's Props live on the capture side, not the base name —
    /// writing to the wrong node makes every fader and DSP knob a no-op.
    #[test]
    fn hardware_strip_controls_its_capture_node() {
        let s = strip(1, StripKind::Hardware);
        assert_eq!(s.control_node(), "lm.strip.1.cap");
        assert_eq!(s.out_node(), "lm.strip.1.out");
    }

    #[test]
    fn bus_node_names_are_lowercase() {
        assert_eq!(BusId::A1.node_base(), "lm.bus.a1");
        assert_eq!(BusId::B2.node_base(), "lm.bus.b2");
    }

    /// A buses meter after their output node; B buses ARE the source node the
    /// world sees, so that is where the post-limiter signal is tapped.
    #[test]
    fn bus_tap_point_depends_on_bus_kind() {
        assert_eq!(bus(BusId::A1, Some("alsa_output.hdmi")).tap_node(), "lm.bus.a1.out");
        assert_eq!(bus(BusId::B1, None).tap_node(), "lm.bus.b1");
        assert_eq!(bus(BusId::A1, Some("x")).in_node(), "lm.bus.a1.in");
    }

    #[test]
    fn strip_node_names_are_unique_per_strip() {
        let a = strip(1, StripKind::Virtual);
        let b = strip(2, StripKind::Virtual);
        assert_ne!(a.out_node(), b.out_node());
        assert_ne!(a.control_node(), b.control_node());
    }

    // ---- IPC contract with src/lib/types.ts --------------------------------

    /// `Target` is untagged: the frontend sends `{"strip": 3}` / `{"bus": "A1"}`.
    /// If this drifts, every command silently fails to deserialize.
    #[test]
    fn target_serializes_untagged() {
        assert_eq!(serde_json::to_string(&Target::Strip { strip: 3 }).unwrap(), r#"{"strip":3}"#);
        assert_eq!(serde_json::to_string(&Target::Bus { bus: BusId::A1 }).unwrap(), r#"{"bus":"A1"}"#);

        assert_eq!(
            serde_json::from_str::<Target>(r#"{"strip":7}"#).unwrap(),
            Target::Strip { strip: 7 }
        );
        assert_eq!(
            serde_json::from_str::<Target>(r#"{"bus":"B2"}"#).unwrap(),
            Target::Bus { bus: BusId::B2 }
        );
    }

    /// The TypeScript mirror expects camelCase. These are the exact keys
    /// `src/lib/types.ts` reads.
    #[test]
    fn app_state_uses_camel_case_keys() {
        let json = serde_json::to_value(AppState {
            strips: vec![strip(1, StripKind::Hardware)],
            buses: vec![bus(BusId::A1, Some("alsa_output.hdmi"))],
            devices: vec![DeviceInfo {
                node_name: "alsa_output.hdmi".into(),
                description: "HDMI".into(),
                media_class: "Audio/Sink".into(),
                serial: 42,
                channels: 2,
            }],
            profile_name: "Default".into(),
            take_default_output: true,
        })
        .expect("serialize");

        assert!(json.get("profileName").is_some());
        assert!(json.get("takeDefaultOutput").is_some());
        assert!(json.get("profile_name").is_none(), "snake_case would break the frontend");

        let s = &json["strips"][0];
        for key in ["hwKey", "gainDb", "kind", "routes", "gate", "comp", "eq"] {
            assert!(s.get(key).is_some(), "strip is missing {key}");
        }
        assert!(json["buses"][0].get("targetHwKey").is_some());
        assert!(json["devices"][0].get("nodeName").is_some());
        assert!(json["devices"][0].get("mediaClass").is_some());
    }

    /// StripKind is snake_case on the wire: types.ts declares "virtual" | "hardware".
    #[test]
    fn strip_kind_is_lowercase_on_the_wire() {
        assert_eq!(serde_json::to_value(StripKind::Virtual).unwrap(), "virtual");
        assert_eq!(serde_json::to_value(StripKind::Hardware).unwrap(), "hardware");
        assert_eq!(serde_json::to_value(StripKind::Vban).unwrap(), "vban");
    }

    #[test]
    fn eq_band_kind_matches_the_typescript_union() {
        assert_eq!(serde_json::to_value(EqBandKind::LowShelf).unwrap(), "low_shelf");
        assert_eq!(serde_json::to_value(EqBandKind::Peak).unwrap(), "peak");
        assert_eq!(serde_json::to_value(EqBandKind::HighShelf).unwrap(), "high_shelf");
    }

    #[test]
    fn dsp_params_use_camel_case_keys() {
        let gate = serde_json::to_value(GateParams::default()).unwrap();
        for key in ["enabled", "thresholdDb", "attackMs", "releaseMs", "holdMs"] {
            assert!(gate.get(key).is_some(), "gate is missing {key}");
        }
        let comp = serde_json::to_value(CompParams::default()).unwrap();
        for key in ["thresholdDb", "ratio", "attackMs", "releaseMs", "makeupDb"] {
            assert!(comp.get(key).is_some(), "comp is missing {key}");
        }
        let lim = serde_json::to_value(LimiterParams::default()).unwrap();
        for key in ["enabled", "thresholdDb", "releaseMs"] {
            assert!(lim.get(key).is_some(), "limiter is missing {key}");
        }
    }

    #[test]
    fn meter_frame_round_trips() {
        let f = MeterFrame { seq: 9, strips: vec![[-6.0, -6.5, -9.0, -9.2]], buses: vec![[0.0; 4]] };
        let back: MeterFrame = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(back, f);
    }

    /// Commands are externally tagged; this is what src-tauri forwards.
    #[test]
    fn engine_commands_round_trip() {
        let cmds = vec![
            EngineCommand::SetGain { target: Target::Strip { strip: 1 }, db: -6.0 },
            EngineCommand::SetRoute { strip: 1, bus: BusId::B1, on: true },
            EngineCommand::SetEqParams { strip: 2, params: EqParams::default() },
            EngineCommand::SetDefaultOutput { on: true },
            EngineCommand::SetMetersEnabled { on: false },
            EngineCommand::SetBusVban { bus: BusId::A1, config: VbanSend::default() },
            EngineCommand::AddVbanStrip { label: "Remote".into(), config: VbanRecv::default() },
            EngineCommand::SetStripVban { strip: 9, label: "Remote".into(), config: VbanRecv::default() },
            EngineCommand::RemoveStrip { strip: 9 },
            EngineCommand::AddStrip { kind: StripKind::Hardware, label: "Guitar".into() },
            EngineCommand::AddBus { output: true },
            EngineCommand::SetBusEqParams { bus: BusId::B2, params: EqParams::bus_default() },
            EngineCommand::SetPanParams { strip: 2, params: PanParams { mono: true, ..Default::default() } },
            EngineCommand::RemoveBus { bus: BusId::A4 },
            EngineCommand::SetLabel { target: Target::Bus { bus: BusId::B3 }, label: "Discord".into() },
            EngineCommand::Shutdown,
        ];
        for cmd in cmds {
            let text = serde_json::to_string(&cmd).expect("serialize");
            serde_json::from_str::<EngineCommand>(&text).expect("deserialize");
        }
    }

    // ---- DSP defaults -----------------------------------------------------

    /// Four user-facing EQ bands plus four reserved for the color pad. The pad
    /// drives indices 4..7 by number, so the count is load-bearing.
    #[test]
    fn eq_defaults_to_eight_bands_split_four_and_four() {
        let eq = EqParams::default();
        assert_eq!(eq.bands.len(), 8);
        assert_eq!(&eq.bands[4..], &EqParams::color_defaults()[..]);
        assert!(!eq.enabled);
    }

    #[test]
    fn eq_panel_bands_span_the_spectrum() {
        let eq = EqParams::default();
        assert_eq!(eq.bands[0].kind, EqBandKind::LowShelf);
        assert_eq!(eq.bands[3].kind, EqBandKind::HighShelf);
        let freqs: Vec<f32> = eq.bands[..4].iter().map(|b| b.freq_hz).collect();
        assert!(freqs.windows(2).all(|w| w[0] < w[1]), "bands must ascend: {freqs:?}");
    }

    /// These are LSP para_equalizer `ft_N` port values; a wrong number silently
    /// selects the wrong filter shape.
    #[test]
    fn lsp_filter_types_are_the_documented_values() {
        assert_eq!(EqBandKind::Peak.lsp_filter_type(), 1.0);
        assert_eq!(EqBandKind::HighShelf.lsp_filter_type(), 3.0);
        assert_eq!(EqBandKind::LowShelf.lsp_filter_type(), 5.0);
    }

    /// The limiter is the last thing before hardware and the virtual mic, so it
    /// defaults to on with headroom.
    #[test]
    fn limiter_defaults_to_enabled_with_headroom() {
        let l = LimiterParams::default();
        assert!(l.enabled);
        assert!(l.threshold_db < 0.0);
    }

    #[test]
    fn gate_and_comp_default_to_bypassed() {
        assert!(!GateParams::default().enabled);
        assert!(!CompParams::default().enabled);
    }

    /// Potato's layout: five hardware outputs then three virtual sources, in
    /// display order. `ALL` and `Ord` must agree, or the rack shuffles buses.
    #[test]
    fn bus_ids_are_five_outputs_then_three_virtual() {
        assert_eq!(BusId::ALL.len(), 8);
        assert!(BusId::ALL.windows(2).all(|w| w[0] < w[1]), "ALL is in display order");
        let outputs: Vec<_> = BusId::ALL.iter().filter(|b| b.is_output()).collect();
        assert_eq!(outputs, [&BusId::A1, &BusId::A2, &BusId::A3, &BusId::A4, &BusId::A5]);
        assert!(!BusId::B3.is_output());
        assert_eq!(BusId::A5.node_base(), "lm.bus.a5");
        assert_eq!(serde_json::to_value(BusId::B3).unwrap(), "B3");
    }

    #[test]
    fn labels_are_trimmed_bounded_and_never_empty() {
        assert_eq!(clean_label("  Game  ").as_deref(), Some("Game"));
        assert_eq!(clean_label("a\tb\nc").as_deref(), Some("abc"), "control characters are dropped");
        assert_eq!(clean_label(&"x".repeat(50)).map(|s| s.len()), Some(LABEL_MAX));
        assert_eq!(clean_label("   "), None);
        assert_eq!(clean_label("Café ♪").as_deref(), Some("Café ♪"), "labels may be any printable text");
    }

    // ---- bus EQ ------------------------------------------------------------

    /// Buses have no color pad, so all eight bands are user bands, flat,
    /// ascending, shelves at the ends, and off until the user turns it on.
    #[test]
    fn bus_eq_defaults_to_eight_flat_ascending_bands() {
        let eq = EqParams::bus_default();
        assert!(!eq.enabled);
        assert_eq!(eq.bands.len(), 8);
        assert!(eq.bands.iter().all(|b| b.gain_db == 0.0));
        assert!(eq.bands.windows(2).all(|w| w[0].freq_hz < w[1].freq_hz));
        assert_eq!(eq.bands[0].kind, EqBandKind::LowShelf);
        assert_eq!(eq.bands[7].kind, EqBandKind::HighShelf);
        assert!(eq.bands[1..7].iter().all(|b| b.kind == EqBandKind::Peak));
    }

    /// An old profile's bus has no `eq` key: it must get the bus layout, not
    /// the strip one (whose upper four bands belong to the color pad).
    #[test]
    fn a_bus_without_eq_gets_the_bus_layout() {
        let mut v = serde_json::to_value(bus(BusId::A1, Some("x"))).unwrap();
        v.as_object_mut().unwrap().remove("eq");
        let b: BusState = serde_json::from_value(v).unwrap();
        assert_eq!(b.eq, EqParams::bus_default());
    }

    // ---- stereo image ------------------------------------------------------

    #[test]
    fn default_pan_is_the_identity() {
        assert_eq!(PanParams::default().matrix(), [[1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn mono_averages_both_channels_into_each_side() {
        let m = PanParams { mono: true, ..Default::default() }.matrix();
        assert_eq!(m, [[0.5, 0.5], [0.5, 0.5]]);
        // Mono wins over reverse: swapping identical channels changes nothing.
        assert_eq!(PanParams { mono: true, reverse: true, balance: 0.0 }.matrix(), m);
    }

    #[test]
    fn reverse_swaps_left_and_right() {
        assert_eq!(PanParams { reverse: true, ..Default::default() }.matrix(), [[0.0, 1.0], [1.0, 0.0]]);
    }

    /// Balance only ever turns the far side down: centre is unity and nothing
    /// is boosted, so panning can't push a strip into clipping.
    #[test]
    fn balance_attenuates_the_opposite_side_only() {
        let left = PanParams { balance: -1.0, ..Default::default() }.matrix();
        assert_eq!(left, [[1.0, 0.0], [0.0, 0.0]], "hard left silences the right channel");
        let half_right = PanParams { balance: 0.5, ..Default::default() }.matrix();
        assert_eq!(half_right, [[0.5, 0.0], [0.0, 1.0]]);
        let wild = PanParams { balance: 7.0, ..Default::default() }.matrix();
        assert_eq!(wild, [[0.0, 0.0], [0.0, 1.0]], "out-of-range balance is clamped");
        let nan = PanParams { balance: f32::NAN, ..Default::default() }.matrix();
        assert_eq!(nan, [[1.0, 0.0], [0.0, 1.0]], "a NaN from a bad profile means centre");
        for p in [left, half_right, wild] {
            assert!(p.iter().flatten().all(|g| (0.0..=1.0).contains(g)));
        }
    }

    #[test]
    fn balance_combines_with_reverse() {
        let m = PanParams { reverse: true, balance: -1.0, mono: false }.matrix();
        assert_eq!(m, [[0.0, 1.0], [0.0, 0.0]], "reversed, then only the left output remains");
    }

    /// Profiles from before panning have no `pan` key, and a partial `pan`
    /// table (hand-edited) fills in the rest.
    #[test]
    fn pan_is_optional_in_saved_state() {
        let mut v = serde_json::to_value(strip(1, StripKind::Virtual)).unwrap();
        v.as_object_mut().unwrap().remove("pan");
        assert_eq!(serde_json::from_value::<StripState>(v.clone()).unwrap().pan, PanParams::default());
        v["pan"] = serde_json::json!({ "mono": true });
        let s: StripState = serde_json::from_value(v).unwrap();
        assert!(s.pan.mono && !s.pan.reverse && s.pan.balance == 0.0);
        assert!(serde_json::to_value(PanParams::default()).unwrap().get("balance").is_some());
    }

    /// Profiles from before the extra buses carry routes for four buses only.
    #[test]
    fn four_bus_routes_still_deserialize() {
        let mut v = serde_json::to_value(strip(1, StripKind::Virtual)).unwrap();
        v["routes"] = serde_json::json!({ "A1": true, "A2": false, "B1": true, "B2": false });
        let s: StripState = serde_json::from_value(v).unwrap();
        assert_eq!(s.routes.get(&BusId::B1), Some(&true));
        assert_eq!(s.routes.get(&BusId::A5), None);
    }

    // ---- VBAN ---------------------------------------------------------------

    /// A VBAN strip has no capture-side node: its own input carries Props,
    /// exactly like a virtual strip. The receiver is a separate stream.
    #[test]
    fn vban_strip_node_names() {
        let s = strip(7, StripKind::Vban);
        assert_eq!(s.control_node(), "lm.strip.7");
        assert_eq!(s.out_node(), "lm.strip.7.out");
        assert_eq!(s.vban_node(), "lm.strip.7.vban");
        assert_eq!(bus(BusId::A2, Some("x")).vban_node(), "lm.bus.a2.vban");
    }

    #[test]
    fn vban_stream_names_fit_the_16_byte_header() {
        assert_eq!(vban_stream_name("Stream1"), "Stream1");
        assert_eq!(vban_stream_name("  padded  "), "padded");
        assert_eq!(vban_stream_name("ABCDEFGHIJKLMNOPQRST"), "ABCDEFGHIJKLMNOP");
        assert_eq!(vban_stream_name("caf\u{e9} mix"), "caf mix", "non-ASCII cannot go on the wire");
        assert_eq!(vban_stream_name("~.*"), ".*", "a leading ~ would become a regex in stream.rules");
        assert_eq!(vban_stream_name(""), "Stream1");
    }

    #[test]
    fn vban_send_requires_a_literal_ip_and_legal_rate() {
        let ok = VbanSend { host: " 192.168.1.20 ".into(), ..VbanSend::default() }.validated().unwrap();
        assert_eq!(ok.host, "192.168.1.20");
        assert!(VbanSend { host: "::1".into(), ..VbanSend::default() }.validated().is_ok());
        assert!(VbanSend { host: "my-pc.local".into(), ..VbanSend::default() }.validated().is_err());
        assert!(VbanSend { port: 0, ..VbanSend::default() }.validated().is_err());
        assert!(VbanSend { rate: 22050, ..VbanSend::default() }.validated().is_err());
    }

    #[test]
    fn vban_recv_sender_filter_is_optional_but_must_be_an_ip() {
        assert!(VbanRecv::default().validated().is_ok(), "empty sender = any");
        assert!(VbanRecv { sender_ip: "10.0.0.5".into(), ..VbanRecv::default() }.validated().is_ok());
        assert!(VbanRecv { sender_ip: "nope".into(), ..VbanRecv::default() }.validated().is_err());
        assert_eq!(VbanRecv { latency_ms: 0, ..VbanRecv::default() }.validated().unwrap().latency_ms, 10);
    }

    #[test]
    fn vban_config_uses_camel_case_keys() {
        let send = serde_json::to_value(VbanSend::default()).unwrap();
        for key in ["enabled", "streamName", "host", "port", "rate"] {
            assert!(send.get(key).is_some(), "vban send is missing {key}");
        }
        let recv = serde_json::to_value(VbanRecv::default()).unwrap();
        for key in ["streamName", "port", "senderIp", "latencyMs"] {
            assert!(recv.get(key).is_some(), "vban recv is missing {key}");
        }
    }

    /// Profiles written before VBAN existed have no `vban` keys at all.
    #[test]
    fn pre_vban_state_still_deserializes() {
        let mut v = serde_json::to_value(bus(BusId::A1, Some("x"))).unwrap();
        v.as_object_mut().unwrap().remove("vban");
        let b: BusState = serde_json::from_value(v).unwrap();
        assert_eq!(b.vban, VbanSend::default());
        assert!(!b.vban.enabled, "VBAN must never start sending on its own");

        let mut v = serde_json::to_value(strip(1, StripKind::Virtual)).unwrap();
        v.as_object_mut().unwrap().remove("vban");
        let s: StripState = serde_json::from_value(v).unwrap();
        assert_eq!(s.vban, None);
    }
}
