<script lang="ts">
  import type { PanParams } from "../lib/types";

  interface Props {
    pan: PanParams;
    onchange: (pan: PanParams) => void;
  }
  let { pan, onchange }: Props = $props();

  let track: HTMLDivElement;
  let dragging = $state(false);

  const clamp = (v: number) => Math.max(-1, Math.min(1, v));
  /** Snap near centre so the middle is easy to hit by hand. */
  const snap = (v: number) => (Math.abs(v) < 0.04 ? 0 : v);

  function setBalance(v: number) {
    const balance = Math.round(snap(clamp(v)) * 100) / 100;
    if (balance !== pan.balance) onchange({ ...pan, balance });
  }

  function fromPointer(e: PointerEvent) {
    const r = track.getBoundingClientRect();
    setBalance(((e.clientX - r.left) / r.width) * 2 - 1);
  }

  function onpointerdown(e: PointerEvent) {
    if (e.button !== 0) return;
    dragging = true;
    track.setPointerCapture(e.pointerId);
    fromPointer(e);
  }
  function onpointermove(e: PointerEvent) {
    if (dragging) fromPointer(e);
  }
  function onpointerup() {
    dragging = false;
  }
  function onkeydown(e: KeyboardEvent) {
    const step = e.shiftKey ? 0.01 : 0.05;
    if (e.key === "ArrowLeft" || e.key === "ArrowDown") setBalance(pan.balance - step);
    else if (e.key === "ArrowRight" || e.key === "ArrowUp") setBalance(pan.balance + step);
    else if (e.key === "Home" || e.key === "0") setBalance(0);
    else return;
    e.preventDefault();
  }
  function onwheel(e: WheelEvent) {
    e.preventDefault();
    setBalance(pan.balance + (e.deltaY < 0 ? 0.05 : -0.05));
  }

  let readout = $derived(
    pan.balance === 0 ? "C" : `${pan.balance < 0 ? "L" : "R"}${Math.round(Math.abs(pan.balance) * 100)}`,
  );
  // Fill from centre toward the knob: left half for L, right half for R.
  let fillLeft = $derived(50 + Math.min(0, pan.balance) * 50);
  let fillWidth = $derived(Math.abs(pan.balance) * 50);
</script>

<div class="panrow">
  <button
    class="pbtn"
    class:active={pan.mono}
    aria-pressed={pan.mono}
    title="Mono: sum left and right"
    onclick={() => onchange({ ...pan, mono: !pan.mono })}
  >
    MONO
  </button>
  <button
    class="pbtn swap"
    class:active={pan.reverse && !pan.mono}
    aria-pressed={pan.reverse}
    disabled={pan.mono}
    title={pan.mono ? "Reverse has no effect in mono" : "Reverse: swap left and right"}
    onclick={() => onchange({ ...pan, reverse: !pan.reverse })}
  >
    <svg width="12" height="8" viewBox="0 0 12 8" aria-hidden="true">
      <path d="M1 2.5h9M7.5 0.5l2.5 2-2.5 2M11 5.5H2M4.5 3.5l-2.5 2 2.5 2" fill="none" stroke="currentColor" stroke-width="1.1" />
    </svg>
  </button>
  <div
    class="track"
    class:dragging
    bind:this={track}
    role="slider"
    tabindex="0"
    aria-label="Balance"
    aria-valuemin={-1}
    aria-valuemax={1}
    aria-valuenow={pan.balance}
    aria-valuetext={readout}
    title={`Balance ${readout} (double-click to centre)`}
    {onpointerdown}
    {onpointermove}
    {onpointerup}
    onpointercancel={onpointerup}
    ondblclick={() => setBalance(0)}
    {onkeydown}
    {onwheel}
  >
    <div class="centre"></div>
    <div class="fill" style:left={`${fillLeft}%`} style:width={`${fillWidth}%`}></div>
    <div class="thumb" style:left={`${50 + pan.balance * 50}%`}></div>
    <span class="readout">{readout}</span>
  </div>
</div>

<style>
  .panrow {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .pbtn {
    flex: none;
    height: 19px;
    padding: 0 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font: 500 9px var(--mono);
    letter-spacing: 0.05em;
    background: var(--bg-3);
    color: var(--text-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
    box-shadow: inset 0 1px 0 var(--hl-top);
  }
  .pbtn.swap {
    width: 22px;
  }
  .pbtn:hover:not(:disabled) {
    background: var(--bg-4);
    color: var(--text-2);
  }
  .pbtn.active {
    background: var(--accent-glow);
    color: var(--accent);
    border-color: var(--accent);
  }
  .pbtn:disabled {
    opacity: 0.35;
  }
  .track {
    position: relative;
    flex: 1;
    min-width: 0;
    height: 19px;
    background: var(--bg-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
    cursor: ew-resize;
    touch-action: none;
    overflow: hidden;
  }
  .track:focus-visible {
    outline: none;
    border-color: var(--accent);
  }
  .centre {
    position: absolute;
    left: 50%;
    top: 3px;
    bottom: 3px;
    width: 1px;
    background: var(--border-1);
  }
  .fill {
    position: absolute;
    top: 7px;
    height: 3px;
    background: var(--accent);
    opacity: 0.55;
  }
  .thumb {
    position: absolute;
    top: 3px;
    bottom: 3px;
    width: 3px;
    margin-left: -1.5px;
    background: var(--text-1);
    border-radius: 1px;
  }
  .track:hover .thumb,
  .track.dragging .thumb {
    background: var(--accent);
  }
  .readout {
    position: absolute;
    right: 3px;
    bottom: 0;
    font: 500 8px var(--mono);
    color: var(--text-3);
    pointer-events: none;
  }
</style>
