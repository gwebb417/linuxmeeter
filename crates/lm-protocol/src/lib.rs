//! Shared types between the audio engine, the Tauri shell, and (via serde)
//! the frontend. Serialization is camelCase to match the TypeScript mirror in
//! `src/lib/types.ts` — keep the two in sync.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BusId {
    A1,
    A2,
    B1,
    B2,
}

impl BusId {
    pub const ALL: [BusId; 4] = [BusId::A1, BusId::A2, BusId::B1, BusId::B2];

    /// Lowercase node-name fragment: `lm.bus.a1`.
    pub fn node_base(&self) -> String {
        format!("lm.bus.{}", format!("{self:?}").to_lowercase())
    }
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
    SetLimiterParams { bus: BusId, params: LimiterParams },
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
    /// Remove a strip. Only VBAN strips can be removed.
    RemoveStrip { strip: StripId },
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

    #[test]
    fn bus_ids_cover_two_hardware_and_two_virtual() {
        assert_eq!(BusId::ALL.len(), 4);
        assert_eq!(BusId::ALL, [BusId::A1, BusId::A2, BusId::B1, BusId::B2]);
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
