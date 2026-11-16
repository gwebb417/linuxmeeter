/** Transient UI state that isn't mixer state (open panels, etc.). */
import type { BusId } from "../types";

class UiStore {
  /** Strip id whose EQ panel is open, or null. */
  eqStrip = $state<number | null>(null);
  /** Bus whose EQ panel is open, or null. */
  eqBus = $state<BusId | null>(null);
  /** Strip id whose Dynamics (gate/comp) panel is open, or null. */
  dynStrip = $state<number | null>(null);
  /** VBAN network streams panel. */
  vbanOpen = $state(false);
}

export const ui = new UiStore();
