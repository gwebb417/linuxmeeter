/** Keyboard shortcut names for macro buttons: "F1", "M", "Ctrl+Shift+2". */

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta", "AltGraph", "CapsLock", "OS"]);

/** The shortcut a key event represents, or null for a bare modifier press. */
export function shortcutName(e: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">): string | null {
  if (MODIFIER_KEYS.has(e.key) || e.key === "Dead" || e.key === "Unidentified") return null;
  let key = e.key === " " ? "Space" : e.key;
  if (key.length === 1) key = key.toUpperCase();
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  // Shift is part of the character for symbols ("!"), so only name it for
  // keys whose name doesn't already change with it.
  if (e.shiftKey && (key.length > 1 || /[A-Z0-9]/.test(key))) parts.push("Shift");
  if (e.metaKey) parts.push("Super");
  parts.push(key);
  return parts.join("+");
}

/** Keys the app itself uses, which macros must not take over. */
export const RESERVED_SHORTCUTS = new Set(["Escape", "Tab", "Shift+Tab", "Enter", "Space"]);

/** True when a key event is meant for a text field, not a shortcut. */
export function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}
