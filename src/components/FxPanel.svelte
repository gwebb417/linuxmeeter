<script lang="ts">
  import type { FxKind, FxState } from "../lib/types";
  import { ECHO_DELAY_MS, ECHO_FEEDBACK_MAX, REVERB_DECAY_MS } from "../lib/types";
  import { mixer } from "../lib/state/mixer.svelte";
  import { ui } from "../lib/state/ui.svelte";
  import Knob from "./Knob.svelte";

  const NAMES: Record<FxKind, string> = { reverb: "REVERB", echo: "ECHO" };
  const HINTS: Record<FxKind, string> = {
    reverb: "a room after the sound: decay length and how dark the tail gets",
    echo: "four repeats: time between them and how much each one fades",
  };

  // Reverb time / damping rebuild the impulse response and reload the effect,
  // so they are sent once a knob has been still for a moment, not per pixel.
  const pending = new Map<FxKind, FxState>();
  const timers = new Map<FxKind, ReturnType<typeof setTimeout>>();
  // What the knobs show while a reverb change is waiting to be sent.
  let draft = $state<Partial<Record<FxKind, FxState>>>({});

  function shown(fx: FxState): FxState {
    return draft[fx.kind] ?? fx;
  }

  function update(fx: FxState, patch: Partial<FxState>, rebuilds = false) {
    const next = { ...shown(fx), ...patch };
    if (!rebuilds) {
      mixer.setFx(next);
      return;
    }
    draft[fx.kind] = next;
    pending.set(fx.kind, next);
    clearTimeout(timers.get(fx.kind));
    timers.set(
      fx.kind,
      setTimeout(() => {
        const v = pending.get(fx.kind);
        pending.delete(fx.kind);
        if (v) mixer.setFx(v);
        delete draft[fx.kind];
      }, 350),
    );
  }

  const fmtDb = (v: number) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`;
  const fmtMs = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(2)} s` : `${Math.round(v)} ms`);
  const fmtPct = (v: number) => `${Math.round(v * 100)}%`;

  function close() {
    ui.fxOpen = false;
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") close();
  }
</script>

<svelte:window {onkeydown} />

<!-- scrim click closes; Esc handled on window -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && close()}>
  <div class="panel" role="dialog" aria-label="Effects">
    <header>
      <div class="title">FX <span class="sub">send knobs live on each strip</span></div>
      <button class="closebtn" title="Close" onclick={close}>
        <svg width="11" height="11" viewBox="0 0 12 12"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.25" /></svg>
      </button>
    </header>

    {#each mixer.fx as fx (fx.kind)}
      {@const v = shown(fx)}
      <section class="fx" aria-label={NAMES[fx.kind]}>
        <div class="sechead">
          <span class="secname">{NAMES[fx.kind]}</span>
          <span class="hint">{HINTS[fx.kind]}</span>
        </div>
        <div class="row">
          <div class="knobs">
            <Knob
              label="RETURN"
              min={-60}
              max={6}
              value={v.returnDb}
              defaultValue={-6}
              fmt={fmtDb}
              onchange={(x) => update(fx, { returnDb: Math.round(x * 10) / 10 })}
            />
            {#if fx.kind === "reverb"}
              <Knob
                label="DECAY"
                min={REVERB_DECAY_MS[0]}
                max={REVERB_DECAY_MS[1]}
                taper="log"
                value={v.timeMs}
                defaultValue={1800}
                fmt={fmtMs}
                onchange={(x) => update(fx, { timeMs: Math.round(x) }, true)}
              />
              <Knob
                label="DAMP"
                min={0}
                max={1}
                value={v.amount}
                defaultValue={0.5}
                fmt={fmtPct}
                onchange={(x) => update(fx, { amount: Math.round(x * 100) / 100 }, true)}
              />
            {:else}
              <Knob
                label="TIME"
                min={ECHO_DELAY_MS[0]}
                max={ECHO_DELAY_MS[1]}
                taper="log"
                value={v.timeMs}
                defaultValue={350}
                fmt={fmtMs}
                onchange={(x) => update(fx, { timeMs: Math.round(x) })}
              />
              <Knob
                label="FEEDBK"
                min={0}
                max={ECHO_FEEDBACK_MAX}
                value={v.amount}
                defaultValue={0.4}
                fmt={fmtPct}
                onchange={(x) => update(fx, { amount: Math.round(x * 100) / 100 })}
              />
            {/if}
          </div>
          <div class="side">
            <button
              class="tbtn mute"
              class:active={fx.mute}
              aria-pressed={fx.mute}
              title={fx.mute ? "Unmute the return" : "Mute the return"}
              onclick={() => update(fx, { mute: !fx.mute })}
            >
              M
            </button>
            <div class="routes" role="group" aria-label="{NAMES[fx.kind]} return buses">
              {#each mixer.buses as b (b.id)}
                <button
                  class="tbtn"
                  class:active={!!fx.routes[b.id]}
                  aria-pressed={!!fx.routes[b.id]}
                  title={`Return to ${b.id} ${b.label}`}
                  onclick={() => mixer.setFxRoute(fx.kind, b.id, !fx.routes[b.id])}
                >
                  {b.id}
                </button>
              {/each}
            </div>
          </div>
        </div>
      </section>
    {/each}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(10, 11, 13, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .panel {
    width: 460px;
    background: var(--bg-2);
    border: 1px solid var(--border-1);
    border-radius: 6px;
    box-shadow: inset 0 1px 0 var(--hl-top), 0 8px 24px rgba(0, 0, 0, 0.6);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  header {
    display: flex;
    align-items: center;
  }
  .title {
    flex: 1;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
  }
  .title .sub {
    color: var(--text-2);
    font-weight: 500;
    margin-left: 6px;
  }
  .closebtn {
    width: 24px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-2);
    border-radius: 2px;
  }
  .closebtn:hover {
    background: var(--mute);
    color: #fff;
  }
  .fx {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--border-0);
  }
  .sechead {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .secname {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-2);
  }
  .hint {
    font-size: 10px;
    color: var(--text-3);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .knobs {
    display: flex;
    gap: 14px;
  }
  .side {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: flex-end;
  }
  .routes {
    display: grid;
    grid-template-columns: repeat(4, 28px);
    gap: 4px;
  }
  .tbtn {
    height: 22px;
    min-width: 0;
    padding: 0;
    font: 500 10px var(--mono);
    letter-spacing: 0.06em;
    background: var(--bg-3);
    color: var(--text-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
    box-shadow: inset 0 1px 0 var(--hl-top);
  }
  .tbtn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
  .tbtn.active {
    background: var(--accent-glow);
    color: var(--accent);
    border-color: var(--accent);
  }
  .tbtn.mute {
    width: 28px;
  }
  .tbtn.mute.active {
    background: var(--mute);
    color: #1a0506;
    border-color: var(--mute);
  }
</style>
