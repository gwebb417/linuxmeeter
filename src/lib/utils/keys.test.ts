import { describe, expect, it } from "vitest";
import { shortcutName } from "./keys";

const ev = (key: string, mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) => ({
  key,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("shortcut names", () => {
  it("names plain and function keys", () => {
    expect(shortcutName(ev("F1"))).toBe("F1");
    expect(shortcutName(ev("m"))).toBe("M");
    expect(shortcutName(ev(" "))).toBe("Space");
  });

  it("puts modifiers first in a fixed order", () => {
    expect(shortcutName(ev("m", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+M");
    expect(shortcutName(ev("F5", { altKey: true, metaKey: true }))).toBe("Alt+Super+F5");
  });

  /// "!" already means Shift+1 on most layouts; naming Shift again would
  /// make the stored shortcut impossible to press.
  it("does not double-count shift on symbols", () => {
    expect(shortcutName(ev("!", { shiftKey: true }))).toBe("!");
    expect(shortcutName(ev("1", { shiftKey: true }))).toBe("Shift+1");
  });

  it("ignores bare modifier presses", () => {
    for (const k of ["Control", "Shift", "Alt", "Meta"]) expect(shortcutName(ev(k))).toBeNull();
  });
});
