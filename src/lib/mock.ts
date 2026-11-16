/** Fake backend: realistic AppState + synthesized 30 fps meter stream.
 *
 * The simulation honors live mixer state (gain, mute, solo, routing), so bus
 * meters actually respond when you pull a fader or punch a route button —
 * design iteration feels like the real app.
 */
import type { AppState, BusId, MacroAction, MacroButton, MeterFrame, StripState, Switch, VbanRecv, VbanSend } from "./types";
import {
  BUS_EQ_BANDS,
  BUS_IDS,
  ECHO_DELAY_MS,
  ECHO_FEEDBACK_MAX,
  FX_KINDS,
  LABEL_MAX,
  MAX_MACRO_ACTIONS,
  MAX_MACROS,
  MAX_STRIPS,
  REVERB_DECAY_MS,
  VBAN_DEFAULT_PORT,
  VBAN_STREAM_NAME_MAX,
  defaultFx,
  defaultPan,
  defaultSends,
  isOutputBus,
} from "./types";
import type { Ipc, Target } from "./ipc";
import { powerSum } from "./utils/db";

type SimKind = "speech" | "music" | "steady";

interface Sim {
  kind: SimKind;
  base: number;
  spread: number;
  phase: number;
  noise: number;
  out: number;
}

function makeSim(kind: SimKind, base: number, spread: number, phase: number): Sim {
  return { kind, base, spread, phase, noise: 0, out: -90 };
}

function simLevel(s: Sim, t: number, dt: number): number {
  s.noise = Math.max(-3, Math.min(3, (s.noise + (Math.random() - 0.5) * dt * 40) * 0.985));
  let db: number;
  if (s.kind === "speech") {
    const active = Math.sin(t * 0.55 + s.phase) + Math.sin(t * 1.7 + s.phase * 2) * 0.5 > -0.1;
    db = active ? s.base + Math.sin(t * 6 + s.phase) * s.spread * 0.4 + s.noise * 2 : -90;
  } else if (s.kind === "music") {
    const beat = Math.pow(Math.max(0, Math.sin(t * Math.PI * 2 * 1.9 + s.phase)), 10) * 7;
    db = s.base + beat + Math.sin(t * 0.23 + s.phase) * s.spread * 0.5 + s.noise;
  } else {
    db = s.base + Math.sin(t * 0.4 + s.phase) * s.spread * 0.5 + s.noise;
  }
  s.out += (db - s.out) * Math.min(1, dt * (db > s.out ? 30 : 9));
  return s.out;
}

const defaultEq = () => ({
  enabled: false,
  bands: [
    { kind: "low_shelf" as const, freqHz: 100, gainDb: 0, q: 0.7 },
    { kind: "peak" as const, freqHz: 400, gainDb: 0, q: 1 },
    { kind: "peak" as const, freqHz: 2500, gainDb: 0, q: 1 },
    { kind: "high_shelf" as const, freqHz: 8000, gainDb: 0, q: 0.7 },
    { kind: "low_shelf" as const, freqHz: 300, gainDb: 0, q: 0.7 },
    { kind: "peak" as const, freqHz: 400, gainDb: 0, q: 0.8 },
    { kind: "peak" as const, freqHz: 3000, gainDb: 0, q: 0.8 },
    { kind: "high_shelf" as const, freqHz: 3500, gainDb: 0, q: 0.7 },
  ],
});
const defaultGate = () => ({ enabled: false, thresholdDb: -40, attackMs: 20, releaseMs: 100, holdMs: 50 });
const defaultComp = () => ({ enabled: false, thresholdDb: -18, ratio: 4, attackMs: 20, releaseMs: 100, makeupDb: 0 });
const defaultLimiter = () => ({ enabled: true, thresholdDb: -1, releaseMs: 50 });
const defaultVbanSend = (bus: BusId): VbanSend => ({
  enabled: false,
  streamName: `linuxmeeter-${bus}`,
  host: "127.0.0.1",
  port: VBAN_DEFAULT_PORT,
  rate: 48000,
});

/** Mirrors lm_protocol::vban_stream_name. */
function vbanStreamName(raw: string): string {
  const cleaned = Array.from(raw.trim().replace(/^~+/, ""))
    .filter((c) => c >= " " && c <= "~")
    .slice(0, VBAN_STREAM_NAME_MAX)
    .join("")
    .trim();
  return cleaned || "Stream1";
}

const IPV4 = /^(25[0-5]|2[0-4]\d|1?\d?\d)(\.(25[0-5]|2[0-4]\d|1?\d?\d)){3}$/;
const looksLikeIp = (s: string) => IPV4.test(s) || (s.includes(":") && /^[0-9a-fA-F:.]+$/.test(s));

const routes = (on: BusId[]): Partial<Record<BusId, boolean>> =>
  Object.fromEntries(BUS_IDS.map((b) => [b, on.includes(b)]));

/** Deep copy of what arrives from the UI. The real backend gets JSON over IPC;
 *  structuredClone would throw on the Svelte state proxies inside. */
const plain = <T,>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

/** Mirrors lm_protocol::MacroButton::validated. */
function validMacro(m: MacroButton): MacroButton {
  const key = m.key?.trim() || null;
  return {
    ...plain(m),
    label: cleanLabel(m.label) ?? `Macro ${m.id}`,
    on: plain(m.on).slice(0, MAX_MACRO_ACTIONS),
    off: plain(m.off).slice(0, MAX_MACRO_ACTIONS),
    key,
    active: m.mode === "push" ? false : m.active,
  };
}

const resolve = (v: Switch, cur: boolean): boolean => (v === "toggle" ? !cur : v === "on");

/** Mirrors Engine::macro_command + handle: run one action against the mock state. */
function runAction(a: MacroAction): void {
  const target = (t: { strip: number } | { bus: BusId }) =>
    "strip" in t ? findStrip(t.strip) : state.buses.find((b) => b.id === t.bus);
  switch (a.type) {
    case "mute": {
      const o = target(a.target);
      if (o) o.mute = resolve(a.value, o.mute);
      break;
    }
    case "solo": {
      const s = findStrip(a.strip);
      if (s) s.solo = resolve(a.value, s.solo);
      break;
    }
    case "route": {
      const s = findStrip(a.strip);
      if (s && state.buses.some((b) => b.id === a.bus)) s.routes[a.bus] = resolve(a.value, !!s.routes[a.bus]);
      break;
    }
    case "gain": {
      const o = target(a.target);
      if (o && Number.isFinite(a.db)) o.gainDb = Math.max(-80, Math.min(12, a.db));
      break;
    }
    case "fx_mute": {
      const f = state.fx.find((x) => x.kind === a.fx);
      if (f) f.mute = resolve(a.value, f.mute);
      break;
    }
    case "vban_send": {
      const b = state.buses.find((x) => x.id === a.bus);
      if (b) b.vban = { ...b.vban, enabled: resolve(a.value, b.vban.enabled) };
      break;
    }
    case "default_output":
      state.takeDefaultOutput = resolve(a.value, state.takeDefaultOutput);
      break;
  }
}

/** Mirrors lm_protocol::clean_label. */
function cleanLabel(raw: string): string | null {
  // eslint-disable-next-line no-control-regex
  const s = Array.from(raw.replace(/[\u0000-\u001f\u007f]/g, "").trim()).slice(0, LABEL_MAX).join("").trim();
  return s || null;
}

const state: AppState = {
  profileName: "Default",
  takeDefaultOutput: false,
  fx: FX_KINDS.map(defaultFx),
  macros: [],
  strips: [
    { id: 1, kind: "hardware", label: "Mic", hwKey: "alsa_input.usb-ZOOM_Corporation_UAC-2", online: true, gainDb: -4, mute: false, solo: false, routes: routes(["A2", "B1"]), gate: defaultGate(), comp: defaultComp(), eq: defaultEq(), pan: defaultPan(), sends: defaultSends(), vban: null },
    { id: 2, kind: "hardware", label: "Line In", hwKey: "alsa_input.pci-0000_31_00.4.analog-stereo", online: true, gainDb: -12, mute: false, solo: false, routes: routes(["A1"]), gate: defaultGate(), comp: defaultComp(), eq: defaultEq(), pan: defaultPan(), sends: defaultSends(), vban: null },
    { id: 3, kind: "virtual", label: "System", hwKey: null, online: true, gainDb: 0, mute: false, solo: false, routes: routes(["A1", "A2"]), gate: defaultGate(), comp: defaultComp(), eq: defaultEq(), pan: defaultPan(), sends: defaultSends(), vban: null },
    { id: 4, kind: "virtual", label: "Music", hwKey: null, online: true, gainDb: -3.2, mute: false, solo: false, routes: routes(["A1", "A2", "B1"]), gate: defaultGate(), comp: defaultComp(), eq: defaultEq(), pan: defaultPan(), sends: defaultSends(), vban: null },
    { id: 5, kind: "virtual", label: "Chat", hwKey: null, online: true, gainDb: -6, mute: false, solo: false, routes: routes(["A2", "B1"]), gate: defaultGate(), comp: defaultComp(), eq: defaultEq(), pan: defaultPan(), sends: defaultSends(), vban: null },
  ],
  buses: [
    { id: "A1", label: "Speakers", targetHwKey: "alsa_output.pci-0000_2f_00.1.hdmi-stereo", online: true, gainDb: 0, mute: false, limiter: defaultLimiter(), eq: { enabled: false, bands: structuredClone(BUS_EQ_BANDS) }, vban: defaultVbanSend("A1") },
    { id: "A2", label: "Headphones", targetHwKey: "alsa_output.usb-ZOOM_Corporation_UAC-2", online: true, gainDb: -2.5, mute: false, limiter: defaultLimiter(), eq: { enabled: false, bands: structuredClone(BUS_EQ_BANDS) }, vban: defaultVbanSend("A2") },
    { id: "B1", label: "Stream Mic", targetHwKey: null, online: true, gainDb: 0, mute: false, limiter: defaultLimiter(), eq: { enabled: false, bands: structuredClone(BUS_EQ_BANDS) }, vban: defaultVbanSend("B1") },
    { id: "B2", label: "Aux", targetHwKey: null, online: true, gainDb: -6, mute: false, limiter: defaultLimiter(), eq: { enabled: false, bands: structuredClone(BUS_EQ_BANDS) }, vban: defaultVbanSend("B2") },
  ],
  devices: [
    { nodeName: "alsa_input.usb-ZOOM_Corporation_UAC-2", description: "ZOOM UAC-2", mediaClass: "Audio/Source", serial: 67, channels: 2 },
    { nodeName: "alsa_input.pci-0000_31_00.4.analog-stereo", description: "Onboard Analog", mediaClass: "Audio/Source", serial: 63, channels: 2 },
    { nodeName: "alsa_input.usb-046d_HD_Pro_Webcam_C920", description: "C920 Webcam Mic", mediaClass: "Audio/Source", serial: 45, channels: 2 },
    { nodeName: "alsa_output.pci-0000_2f_00.1.hdmi-stereo", description: "HDMI / DisplayPort", mediaClass: "Audio/Sink", serial: 65, channels: 2 },
    { nodeName: "alsa_output.usb-ZOOM_Corporation_UAC-2", description: "ZOOM UAC-2", mediaClass: "Audio/Sink", serial: 66, channels: 2 },
  ],
};

const sims = new Map<number, Sim>([
  [1, makeSim("speech", -24, 9, 1.7)],
  [2, makeSim("steady", -36, 3, 4.1)],
  [3, makeSim("steady", -30, 6, 2.3)],
  [4, makeSim("music", -11, 4, 0.4)],
  [5, makeSim("speech", -22, 8, 3.6)],
]);

function findStrip(id: number): StripState | undefined {
  return state.strips.find((s) => s.id === id);
}

let mockAutostart = false;

const meterSubs = new Set<(f: MeterFrame) => void>();
const stateSubs = new Set<(s: AppState) => void>();
const errorSubs = new Set<(m: string) => void>();

/** Push a snapshot, as the engine does after structural changes. */
function broadcast(): void {
  for (const cb of stateSubs) cb(structuredClone(state));
}

/** Reject like the engine: emit an error and resend the unchanged state. */
function reject(message: string): Promise<void> {
  for (const cb of errorSubs) cb(message);
  broadcast();
  return Promise.resolve();
}

function checkRecv(config: VbanRecv, except: number | null): string | VbanRecv {
  const senderIp = config.senderIp.trim();
  if (senderIp && !looksLikeIp(senderIp)) return `VBAN input: "${senderIp}" is not an IP address`;
  if (!(config.port >= 1 && config.port <= 65535)) return "VBAN input: port must be 1-65535";
  const cfg: VbanRecv = {
    streamName: vbanStreamName(config.streamName),
    port: config.port,
    senderIp,
    latencyMs: Math.min(2000, Math.max(10, config.latencyMs)),
  };
  const clash = state.strips.find(
    (s) =>
      s.id !== except &&
      s.vban &&
      s.vban.port === cfg.port &&
      s.vban.streamName === cfg.streamName &&
      (!s.vban.senderIp || !cfg.senderIp || s.vban.senderIp === cfg.senderIp),
  );
  if (clash) return `VBAN input: strip "${clash.label}" already receives "${cfg.streamName}" on port ${cfg.port}`;
  return cfg;
}
let seq = 0;
let timer: ReturnType<typeof setInterval> | undefined;
let lastTick = 0;

function startSim(): void {
  if (timer) return;
  lastTick = performance.now();
  timer = setInterval(() => {
    const now = performance.now();
    const t = now / 1000;
    const dt = Math.min(0.1, (now - lastTick) / 1000);
    lastTick = now;

    const anySolo = state.strips.some((s) => s.solo);
    const levels = new Map<number, number>();
    const stripFrames: number[][] = state.strips.map((s) => {
      const raw = simLevel(sims.get(s.id)!, t, dt);
      const audible = !s.mute && (!anySolo || s.solo) && s.online;
      const lvl = audible && s.gainDb !== -Infinity ? raw + s.gainDb : -90;
      levels.set(s.id, lvl);
      const jitter = () => Math.random() * 1.2;
      // Balance only attenuates the far side (lm_protocol::PanParams::matrix).
      const b = s.pan.balance;
      const side = (g: number) => (g <= 0 ? -90 : 20 * Math.log10(g));
      const [gl, gr] = [side(Math.min(1, 1 - b)), side(Math.min(1, 1 + b))];
      return [lvl + 0.6 + jitter() + gl, lvl - 0.6 + jitter() + gr, lvl - 2.5 + gl, lvl - 2.8 + gr];
    });
    const busFrames: number[][] = state.buses.map((b) => {
      const inputs = state.strips.filter((s) => s.routes[b.id]).map((s) => levels.get(s.id)!);
      // Effect returns routed here carry each strip's send (roughly: level x send x return).
      const returns = state.fx
        .filter((f) => !f.mute && f.routes[b.id])
        .flatMap((f) =>
          state.strips
            .filter((s) => s.sends[f.kind] > 0)
            .map((s) => levels.get(s.id)! + 20 * Math.log10(s.sends[f.kind]) + f.returnDb),
        );
      const sum = b.mute ? -90 : powerSum([...inputs, ...returns]) + b.gainDb;
      return [sum + 0.4, sum - 0.4, sum - 2.5, sum - 2.7];
    });

    const frame: MeterFrame = { seq: seq++, strips: stripFrames, buses: busFrames };
    for (const cb of meterSubs) cb(frame);
  }, 33);
}

function target(t: Target): { gainDb: number; mute: boolean } | undefined {
  return "strip" in t ? findStrip(t.strip) : state.buses.find((b) => b.id === t.bus);
}

export const mockIpc: Ipc = {
  getState: () => Promise.resolve(structuredClone(state)),
  setGain: (t, db) => {
    const o = target(t);
    if (o) o.gainDb = db;
    return Promise.resolve();
  },
  setMute: (t, mute) => {
    const o = target(t);
    if (o) o.mute = mute;
    return Promise.resolve();
  },
  setSolo: (stripId, solo) => {
    const s = findStrip(stripId);
    if (s) s.solo = solo;
    return Promise.resolve();
  },
  setRoute: (stripId, bus, on) => {
    const s = findStrip(stripId);
    if (s) s.routes[bus] = on;
    return Promise.resolve();
  },
  setGateParams: (stripId, params) => {
    const s = findStrip(stripId);
    if (s) s.gate = { ...params };
    return Promise.resolve();
  },
  setCompParams: (stripId, params) => {
    const s = findStrip(stripId);
    if (s) s.comp = { ...params };
    return Promise.resolve();
  },
  setEqParams: (stripId, params) => {
    const s = findStrip(stripId);
    if (s) s.eq = plain(params);
    return Promise.resolve();
  },
  setPanParams: (stripId, params) => {
    const s = findStrip(stripId);
    if (s) s.pan = { ...params, balance: Math.max(-1, Math.min(1, params.balance)) };
    return Promise.resolve();
  },
  setMacros: (macros) => {
    const seen = new Set<number>();
    state.macros = macros
      .filter((m) => !seen.has(m.id) && (seen.add(m.id), true))
      .map(validMacro)
      .slice(0, MAX_MACROS);
    broadcast();
    return Promise.resolve();
  },
  triggerMacro: (id, pressed) => {
    const m = state.macros.find((x) => x.id === id);
    if (!m) return Promise.resolve();
    let run: MacroAction[] | null = null;
    if (m.mode === "toggle" && pressed) {
      m.active = !m.active;
      run = m.active ? m.on : m.off;
    } else if (m.mode === "push" && pressed && !m.active) {
      m.active = true;
      run = m.on;
    } else if (m.mode === "push" && !pressed && m.active) {
      m.active = false;
      run = m.off;
    }
    for (const a of run ?? []) runAction(a);
    broadcast();
    return Promise.resolve();
  },
  setSends: (stripId, sends) => {
    const s = findStrip(stripId);
    const c = (v: number) => (Number.isFinite(v) ? Math.max(0, Math.min(1, v)) : 0);
    if (s) s.sends = { reverb: c(sends.reverb), echo: c(sends.echo) };
    return Promise.resolve();
  },
  setFx: (fx) => {
    const cur = state.fx.find((f) => f.kind === fx.kind);
    if (!cur) return Promise.resolve();
    // Mirrors lm_protocol::FxState::validated.
    const [tmin, tmax] = fx.kind === "reverb" ? REVERB_DECAY_MS : ECHO_DELAY_MS;
    const amax = fx.kind === "reverb" ? 1 : ECHO_FEEDBACK_MAX;
    const clamp = (v: number, lo: number, hi: number, d: number) => (Number.isFinite(v) ? Math.max(lo, Math.min(hi, v)) : d);
    const d = defaultFx(fx.kind);
    Object.assign(cur, {
      returnDb: clamp(fx.returnDb, -60, 6, d.returnDb),
      mute: fx.mute,
      timeMs: clamp(fx.timeMs, tmin, tmax, d.timeMs),
      amount: clamp(fx.amount, 0, amax, d.amount),
    });
    broadcast();
    return Promise.resolve();
  },
  setFxRoute: (kind, bus, on) => {
    const f = state.fx.find((x) => x.kind === kind);
    if (f && state.buses.some((b) => b.id === bus)) f.routes[bus] = on;
    broadcast();
    return Promise.resolve();
  },
  setBusEqParams: (bus, params) => {
    const b = state.buses.find((x) => x.id === bus);
    if (b) b.eq = plain(params);
    return Promise.resolve();
  },
  setLimiterParams: (bus, params) => {
    const b = state.buses.find((x) => x.id === bus);
    if (b) b.limiter = { ...params };
    return Promise.resolve();
  },
  setStripDevice: (stripId, hwKey) => {
    const s = findStrip(stripId);
    if (s) s.hwKey = hwKey;
    return Promise.resolve();
  },
  setBusTarget: (bus, hwKey) => {
    const b = state.buses.find((x) => x.id === bus);
    if (b) b.targetHwKey = hwKey;
    return Promise.resolve();
  },
  setBusVban: (bus, config) => {
    const b = state.buses.find((x) => x.id === bus);
    if (!b) return Promise.resolve();
    if (config.enabled) {
      const host = config.host.trim();
      if (!looksLikeIp(host)) return reject(`VBAN output: "${host}" is not an IP address`);
      if (!(config.port >= 1 && config.port <= 65535)) return reject("VBAN output: port must be 1-65535");
      b.vban = { ...config, host, streamName: vbanStreamName(config.streamName) };
    } else {
      b.vban = { ...config, enabled: false };
    }
    broadcast();
    return Promise.resolve();
  },
  addVbanStrip: (label, config) => {
    const cfg = checkRecv(config, null);
    if (typeof cfg === "string") return reject(cfg);
    const id = Math.max(0, ...state.strips.map((s) => s.id)) + 1;
    state.strips.push({
      id,
      kind: "vban",
      label: label.trim() || cfg.streamName,
      hwKey: null,
      // Pretend a sender shows up shortly after, like a real network peer.
      online: false,
      gainDb: 0,
      mute: false,
      solo: false,
      routes: routes(["A1"]),
      gate: defaultGate(),
      comp: defaultComp(),
      eq: defaultEq(), pan: defaultPan(), sends: defaultSends(),
      vban: cfg,
    });
    sims.set(id, makeSim("music", -16, 5, Math.random() * 6));
    broadcast();
    setTimeout(() => {
      const s = findStrip(id);
      if (s) {
        s.online = true;
        broadcast();
      }
    }, 1200);
    return Promise.resolve();
  },
  setStripVban: (stripId, label, config) => {
    const s = findStrip(stripId);
    if (!s || s.kind !== "vban") return Promise.resolve();
    const cfg = checkRecv(config, stripId);
    if (typeof cfg === "string") return reject(cfg);
    if (label.trim()) s.label = label.trim();
    s.vban = cfg;
    broadcast();
    return Promise.resolve();
  },
  removeStrip: (stripId) => {
    const idx = state.strips.findIndex((s) => s.id === stripId);
    if (idx < 0) return Promise.resolve();
    state.strips.splice(idx, 1);
    sims.delete(stripId);
    broadcast();
    return Promise.resolve();
  },
  addStrip: (kind, label) => {
    if (state.strips.length >= MAX_STRIPS) return reject(`the mixer is limited to ${MAX_STRIPS} strips`);
    const id = Math.max(0, ...state.strips.map((s) => s.id)) + 1;
    const n = state.strips.filter((s) => s.kind === kind).length + 1;
    const hwKey = kind === "hardware" ? (state.devices.find((d) => d.mediaClass === "Audio/Source")?.nodeName ?? null) : null;
    if (kind === "hardware" && !hwKey) return reject("no input device to add a hardware strip for");
    state.strips.push({
      id,
      kind,
      label: cleanLabel(label) ?? (kind === "hardware" ? `Input ${n}` : `Virtual ${n}`),
      hwKey,
      online: true,
      gainDb: 0,
      mute: false,
      solo: false,
      routes: routes([]),
      gate: defaultGate(),
      comp: defaultComp(),
      eq: defaultEq(), pan: defaultPan(), sends: defaultSends(),
      vban: null,
    });
    sims.set(id, makeSim(kind === "hardware" ? "speech" : "music", -20, 6, Math.random() * 6));
    broadcast();
    return Promise.resolve();
  },
  addBus: (output) => {
    const id = BUS_IDS.find((b) => isOutputBus(b) === output && !state.buses.some((x) => x.id === b));
    if (!id) return reject(output ? "all five A buses are in use" : "all three B buses are in use");
    const targetHwKey = output ? (state.devices.find((d) => d.mediaClass === "Audio/Sink")?.nodeName ?? null) : null;
    const bus = {
      id,
      label: `${output ? "Output" : "Virtual"} ${id}`,
      targetHwKey,
      online: true,
      gainDb: 0,
      mute: false,
      limiter: defaultLimiter(), eq: { enabled: false, bands: structuredClone(BUS_EQ_BANDS) },
      vban: defaultVbanSend(id),
    };
    state.buses.push(bus);
    state.buses.sort((a, b) => BUS_IDS.indexOf(a.id) - BUS_IDS.indexOf(b.id));
    for (const s of state.strips) s.routes[id] ??= false;
    for (const f of state.fx) f.routes[id] ??= false;
    broadcast();
    return Promise.resolve();
  },
  removeBus: (bus) => {
    if (state.buses.length === 1) return reject("the mixer needs at least one bus");
    const idx = state.buses.findIndex((b) => b.id === bus);
    if (idx < 0) return Promise.resolve();
    state.buses.splice(idx, 1);
    for (const s of state.strips) delete s.routes[bus];
    for (const f of state.fx) delete f.routes[bus];
    broadcast();
    return Promise.resolve();
  },
  setLabel: (t, raw) => {
    const label = cleanLabel(raw);
    if (!label) return reject("a label can't be empty");
    const o = "strip" in t ? findStrip(t.strip) : state.buses.find((b) => b.id === t.bus);
    if (o) o.label = label;
    broadcast();
    return Promise.resolve();
  },
  setDefaultOutput: (on) => {
    state.takeDefaultOutput = on;
    return Promise.resolve();
  },
  getAutostart: () => Promise.resolve(mockAutostart),
  setAutostart: (on) => {
    mockAutostart = on;
    return Promise.resolve();
  },
  onMeters: (cb) => {
    meterSubs.add(cb);
    startSim();
    return () => meterSubs.delete(cb);
  },
  onStateChanged: (cb) => {
    stateSubs.add(cb);
    return () => stateSubs.delete(cb);
  },
  onError: (cb) => {
    errorSubs.add(cb);
    return () => errorSubs.delete(cb);
  },
};
