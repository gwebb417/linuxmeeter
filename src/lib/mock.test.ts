import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Ipc } from "./ipc";

/** The mock is the design-iteration backend; it must follow the engine's
 *  population rules or the browser preview lies about what the app does.
 *  Each test gets a fresh module so state doesn't leak between them. */
let ipc: Ipc;
let errors: string[];

beforeEach(async () => {
  vi.resetModules();
  ({ mockIpc: ipc } = await import("./mock"));
  errors = [];
  ipc.onError((m) => errors.push(m));
});

const busIds = async () => (await ipc.getState()).buses.map((b) => b.id);

describe("bus population", () => {
  it("adds the next free A or B bus and keeps A1..B3 order", async () => {
    await ipc.addBus(true);
    await ipc.addBus(false);
    expect(await busIds()).toEqual(["A1", "A2", "A3", "B1", "B2", "B3"]);
  });

  it("refills a gap before using a higher slot", async () => {
    await ipc.addBus(true); // A3
    await ipc.removeBus("A2");
    await ipc.addBus(true);
    expect(await busIds()).toEqual(["A1", "A2", "A3", "B1", "B2"]);
  });

  it("stops at five A buses and three B buses", async () => {
    for (let i = 0; i < 4; i++) await ipc.addBus(true);
    for (let i = 0; i < 2; i++) await ipc.addBus(false);
    expect(await busIds()).toEqual(["A1", "A2", "A3", "A4", "A5", "B1", "B2", "B3"]);
    expect(errors).toEqual(["all five A buses are in use", "all three B buses are in use"]);
  });

  it("new A buses get an output device, new B buses are virtual", async () => {
    await ipc.addBus(true);
    await ipc.addBus(false);
    const s = await ipc.getState();
    expect(s.buses.find((b) => b.id === "A3")?.targetHwKey).toBeTruthy();
    expect(s.buses.find((b) => b.id === "B3")?.targetHwKey).toBeNull();
  });

  it("removing a bus drops every route into it", async () => {
    await ipc.removeBus("B1");
    const s = await ipc.getState();
    expect(s.buses.some((b) => b.id === "B1")).toBe(false);
    expect(s.strips.every((x) => !("B1" in x.routes))).toBe(true);
  });

  it("keeps at least one bus", async () => {
    for (const b of ["A2", "B1", "B2"] as const) await ipc.removeBus(b);
    await ipc.removeBus("A1");
    expect(await busIds()).toEqual(["A1"]);
    expect(errors).toEqual(["the mixer needs at least one bus"]);
  });
});

describe("strip population", () => {
  it("adds hardware and virtual strips with nothing routed", async () => {
    await ipc.addStrip("virtual", "");
    await ipc.addStrip("hardware", "Guitar");
    const s = await ipc.getState();
    const [v, h] = s.strips.slice(-2);
    if (!v || !h) throw new Error("expected two new strips");
    expect(v.kind).toBe("virtual");
    expect(v.label).toBe("Virtual 4");
    expect(h.kind).toBe("hardware");
    expect(h.label).toBe("Guitar");
    expect(h.hwKey).toBeTruthy();
    expect(Object.values(v.routes).some(Boolean)).toBe(false);
    expect(new Set(s.strips.map((x) => x.id)).size).toBe(s.strips.length);
  });

  it("removes strips of any kind", async () => {
    await ipc.removeStrip(1);
    await ipc.removeStrip(3);
    expect((await ipc.getState()).strips.map((x) => x.id)).toEqual([2, 4, 5]);
  });

  it("caps the rack at sixteen strips", async () => {
    for (let i = 0; i < 12; i++) await ipc.addStrip("virtual", "");
    expect((await ipc.getState()).strips.length).toBe(16);
    expect(errors).toEqual(["the mixer is limited to 16 strips"]);
  });
});

describe("EQ edits", () => {
  /** The UI passes objects that may wrap reactive proxies; the mock must copy
   *  them, not keep references the UI can mutate behind its back. */
  it("stores a detached copy of a bus EQ", async () => {
    const s = await ipc.getState();
    const eq = s.buses[0]!.eq;
    const edited = { ...eq, enabled: true, bands: eq.bands.map((b, i) => (i === 3 ? { ...b, gainDb: 6 } : b)) };
    await ipc.setBusEqParams("A1", edited);
    edited.bands[3]!.gainDb = -20;
    const after = (await ipc.getState()).buses[0]!.eq;
    expect(after.enabled).toBe(true);
    expect(after.bands[3]!.gainDb).toBe(6);
  });

  it("gives every bus eight EQ bands", async () => {
    await ipc.addBus(true);
    expect((await ipc.getState()).buses.every((b) => b.eq.bands.length === 8)).toBe(true);
  });
});

describe("effects", () => {
  it("starts with both effects returning to A1 and no sends", async () => {
    const s = await ipc.getState();
    expect(s.fx.map((f) => f.kind)).toEqual(["reverb", "echo"]);
    expect(s.fx.every((f) => f.routes.A1 && !f.mute)).toBe(true);
    expect(s.strips.every((x) => x.sends.reverb === 0 && x.sends.echo === 0)).toBe(true);
  });

  it("clamps settings like the engine", async () => {
    await ipc.setFx({ kind: "echo", returnDb: 40, mute: false, routes: {}, timeMs: 99999, amount: 3 });
    await ipc.setSends(3, { reverb: 1.8, echo: -1 });
    const s = await ipc.getState();
    const echo = s.fx.find((f) => f.kind === "echo")!;
    expect(echo.returnDb).toBe(6);
    expect(echo.timeMs).toBe(1500);
    expect(echo.amount).toBe(0.9);
    expect(echo.routes.A1).toBe(true); // setFx leaves routing alone
    expect(s.strips.find((x) => x.id === 3)!.sends).toEqual({ reverb: 1, echo: 0 });
  });

  it("routes returns only to buses that exist, and forgets removed ones", async () => {
    await ipc.setFxRoute("reverb", "A5", true); // no A5 yet
    await ipc.setFxRoute("reverb", "B1", true);
    await ipc.removeBus("B1");
    const reverb = (await ipc.getState()).fx[0]!;
    expect(reverb.routes.A5).toBeUndefined();
    expect("B1" in reverb.routes).toBe(false);
  });
});

describe("macro buttons", () => {
  const talk = {
    id: 1,
    label: "Talk",
    mode: "push" as const,
    on: [{ type: "route" as const, strip: 3, bus: "B1" as const, value: "on" as const }],
    off: [{ type: "route" as const, strip: 3, bus: "B1" as const, value: "off" as const }],
    key: " F1 ",
    active: true,
  };

  it("stores a normalized list", async () => {
    await ipc.setMacros([talk, { ...talk, label: "dupe" }, { ...talk, id: 2, label: "  " }]);
    const m = (await ipc.getState()).macros;
    expect(m.map((x) => x.id)).toEqual([1, 2]);
    expect(m[0]!.key).toBe("F1");
    expect(m[0]!.active).toBe(false);
    expect(m[1]!.label).toBe("Macro 2");
  });

  it("push buttons act while held, ignoring repeats and stray releases", async () => {
    await ipc.setMacros([talk]);
    const routed = async () => (await ipc.getState()).strips.find((s) => s.id === 3)!.routes.B1;
    await ipc.triggerMacro(1, true);
    expect(await routed()).toBe(true);
    await ipc.triggerMacro(1, true); // repeat
    await ipc.triggerMacro(1, false);
    expect(await routed()).toBe(false);
    expect((await ipc.getState()).macros[0]!.active).toBe(false);
  });

  it("toggle buttons alternate their two lists", async () => {
    await ipc.setMacros([
      {
        id: 5,
        label: "Mute all",
        mode: "toggle",
        on: [{ type: "mute", target: { bus: "A1" }, value: "toggle" }, { type: "gain", target: { strip: 3 }, db: -99 }],
        off: [{ type: "mute", target: { bus: "A1" }, value: "toggle" }],
        key: null,
        active: false,
      },
    ]);
    await ipc.triggerMacro(5, true);
    let s = await ipc.getState();
    expect(s.buses[0]!.mute).toBe(true);
    expect(s.strips.find((x) => x.id === 3)!.gainDb).toBe(-80); // clamped
    expect(s.macros[0]!.active).toBe(true);
    await ipc.triggerMacro(5, false); // releasing a toggle does nothing
    await ipc.triggerMacro(5, true);
    s = await ipc.getState();
    expect(s.buses[0]!.mute).toBe(false);
    expect(s.macros[0]!.active).toBe(false);
  });
});

describe("labels", () => {
  it("renames strips and buses, trimmed and bounded", async () => {
    await ipc.setLabel({ strip: 3 }, "  Game  ");
    await ipc.setLabel({ bus: "B1" }, "x".repeat(40));
    const s = await ipc.getState();
    expect(s.strips.find((x) => x.id === 3)?.label).toBe("Game");
    expect(s.buses.find((b) => b.id === "B1")?.label).toHaveLength(32);
  });

  it("rejects an empty label and keeps the old one", async () => {
    await ipc.setLabel({ strip: 3 }, "   ");
    expect((await ipc.getState()).strips.find((x) => x.id === 3)?.label).toBe("System");
    expect(errors).toEqual(["a label can't be empty"]);
  });
});
