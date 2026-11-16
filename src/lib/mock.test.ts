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
