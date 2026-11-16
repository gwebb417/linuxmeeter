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
