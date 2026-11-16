/** Mirrors crates/lm-protocol (serde-renamed to camelCase at the IPC boundary). */

/** VoiceMeeter Potato's bus slots: A = hardware outputs, B = virtual sources.
 *  A profile holds any subset, in this order. */
export type BusId = "A1" | "A2" | "A3" | "A4" | "A5" | "B1" | "B2" | "B3";
export const BUS_IDS: BusId[] = ["A1", "A2", "A3", "A4", "A5", "B1", "B2", "B3"];
export const isOutputBus = (id: BusId): boolean => id.startsWith("A");

/** Mirrors lm_protocol::MAX_STRIPS / LABEL_MAX. */
export const MAX_STRIPS = 16;
export const LABEL_MAX = 32;

export type StripKind = "virtual" | "hardware" | "vban";

export const VBAN_DEFAULT_PORT = 6980;
export const VBAN_STREAM_NAME_MAX = 16;
export const VBAN_RATES = [44100, 48000, 96000] as const;

/** Incoming VBAN stream feeding a "vban" strip. */
export interface VbanRecv {
  /** Only this stream name is accepted (PipeWire ≥ 1.4). */
  streamName: string;
  /** Local UDP port to listen on. */
  port: number;
  /** Only accept packets from this sender; "" = any. */
  senderIp: string;
  latencyMs: number;
}

/** Outgoing VBAN stream carrying a bus's post-limiter mix. */
export interface VbanSend {
  enabled: boolean;
  streamName: string;
  /** Literal IP address (PipeWire does not resolve hostnames). */
  host: string;
  port: number;
  rate: number;
}

export interface GateParams {
  enabled: boolean;
  thresholdDb: number;
  attackMs: number;
  releaseMs: number;
  holdMs: number;
}

export interface CompParams {
  enabled: boolean;
  thresholdDb: number;
  ratio: number;
  attackMs: number;
  releaseMs: number;
  makeupDb: number;
}

export interface LimiterParams {
  enabled: boolean;
  thresholdDb: number;
  releaseMs: number;
}

export type EqBandKind = "low_shelf" | "peak" | "high_shelf";

export interface EqBand {
  kind: EqBandKind;
  freqHz: number;
  gainDb: number;
  q: number;
}

export interface EqParams {
  enabled: boolean;
  bands: EqBand[];
}

/** The four EQ-panel bands of a strip (bands 4..7 belong to the color pad).
 *  Mirrors the first half of lm_protocol::EqParams::default. */
export const STRIP_EQ_BANDS: EqBand[] = [
  { kind: "low_shelf", freqHz: 100, gainDb: 0, q: 0.7 },
  { kind: "peak", freqHz: 400, gainDb: 0, q: 1 },
  { kind: "peak", freqHz: 2500, gainDb: 0, q: 1 },
  { kind: "high_shelf", freqHz: 8000, gainDb: 0, q: 0.7 },
];

/** A bus EQ: all eight bands are user bands. Mirrors lm_protocol::EqParams::bus_default. */
export const BUS_EQ_BANDS: EqBand[] = [
  { kind: "low_shelf", freqHz: 60, gainDb: 0, q: 0.7 },
  { kind: "peak", freqHz: 150, gainDb: 0, q: 1 },
  { kind: "peak", freqHz: 400, gainDb: 0, q: 1 },
  { kind: "peak", freqHz: 1000, gainDb: 0, q: 1 },
  { kind: "peak", freqHz: 2500, gainDb: 0, q: 1 },
  { kind: "peak", freqHz: 5000, gainDb: 0, q: 1 },
  { kind: "peak", freqHz: 8000, gainDb: 0, q: 1 },
  { kind: "high_shelf", freqHz: 12000, gainDb: 0, q: 0.7 },
];

/** Macro buttons (mirror lm_protocol's macro types). */
export const MAX_MACROS = 24;
export const MAX_MACRO_ACTIONS = 32;
export type Switch = "on" | "off" | "toggle";
export type MacroMode = "toggle" | "push";
export type MacroAction =
  | { type: "mute"; target: { strip: number } | { bus: BusId }; value: Switch }
  | { type: "solo"; strip: number; value: Switch }
  | { type: "route"; strip: number; bus: BusId; value: Switch }
  | { type: "gain"; target: { strip: number } | { bus: BusId }; db: number }
  | { type: "fx_mute"; fx: FxKind; value: Switch }
  | { type: "vban_send"; bus: BusId; value: Switch }
  | { type: "default_output"; value: Switch };

export interface MacroButton {
  id: number;
  label: string;
  mode: MacroMode;
  /** Run on turning on (toggle) / on press (push). */
  on: MacroAction[];
  /** Run on turning off (toggle) / on release (push). */
  off: MacroAction[];
  /** In-window shortcut, e.g. "F1" or "Ctrl+M". */
  key: string | null;
  active: boolean;
}

/** The shared effects (mirrors lm_protocol::FxKind). */
export type FxKind = "reverb" | "echo";
export const FX_KINDS: FxKind[] = ["reverb", "echo"];

/** A strip's send levels, linear 0..1, post-fader and post-pan. */
export interface FxSends {
  reverb: number;
  echo: number;
}
export const defaultSends = (): FxSends => ({ reverb: 0, echo: 0 });

/** Mirrors lm_protocol::{REVERB_DECAY_MS, ECHO_DELAY_MS, ECHO_FEEDBACK_MAX}. */
export const REVERB_DECAY_MS = [300, 6000] as const;
export const ECHO_DELAY_MS = [20, 1500] as const;
export const ECHO_FEEDBACK_MAX = 0.9;

/** One shared effect (mirrors lm_protocol::FxState). */
export interface FxState {
  kind: FxKind;
  returnDb: number;
  mute: boolean;
  routes: Partial<Record<BusId, boolean>>;
  /** Reverb: decay (RT60). Echo: time between repeats. */
  timeMs: number;
  /** Reverb: damping 0..1. Echo: feedback 0..ECHO_FEEDBACK_MAX. */
  amount: number;
}

export const defaultFx = (kind: FxKind): FxState => ({
  kind,
  returnDb: -6,
  mute: false,
  routes: { A1: true },
  timeMs: kind === "reverb" ? 1800 : 350,
  amount: kind === "reverb" ? 0.5 : 0.4,
});

/** A strip's stereo image, applied after the EQ (mirrors lm_protocol::PanParams). */
export interface PanParams {
  /** Sum to mono (wins over reverse). */
  mono: boolean;
  /** Swap left and right. */
  reverse: boolean;
  /** -1 hard left … 0 centre … 1 hard right; only ever attenuates. */
  balance: number;
}

export const defaultPan = (): PanParams => ({ mono: false, reverse: false, balance: 0 });

export interface StripState {
  id: number;
  kind: StripKind;
  label: string;
  /** Stable PipeWire identity (node.name) for hardware strips. */
  hwKey: string | null;
  online: boolean;
  gainDb: number;
  mute: boolean;
  solo: boolean;
  /** Only buses that exist (or once existed) have an entry; missing = off. */
  routes: Partial<Record<BusId, boolean>>;
  gate: GateParams;
  comp: CompParams;
  eq: EqParams;
  pan: PanParams;
  sends: FxSends;
  /** Receiver config; non-null exactly for "vban" strips. */
  vban: VbanRecv | null;
}

export interface BusState {
  id: BusId;
  label: string;
  targetHwKey: string | null;
  online: boolean;
  gainDb: number;
  mute: boolean;
  limiter: LimiterParams;
  /** Eight user bands, before the limiter. */
  eq: EqParams;
  vban: VbanSend;
}

export interface DeviceInfo {
  nodeName: string;
  description: string;
  mediaClass: string;
  serial: number;
  channels: number;
}

export interface AppState {
  strips: StripState[];
  buses: BusState[];
  devices: DeviceInfo[];
  profileName: string;
  /** True while linuxmeeter's System strip is the system default output. */
  takeDefaultOutput: boolean;
  /** Reverb then echo. */
  fx: FxState[];
  macros: MacroButton[];
}

/** One batched meter update, ~30/s. Channel layout: [peakL, peakR, rmsL, rmsR] in dBFS. */
export interface MeterFrame {
  seq: number;
  /** Keyed by strip id order in AppState.strips. */
  strips: number[][];
  /** Keyed by bus order in AppState.buses. */
  buses: number[][];
}

export type MeterKey = `s:${number}` | `b:${BusId}`;
