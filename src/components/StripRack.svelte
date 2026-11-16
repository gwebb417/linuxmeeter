<script lang="ts">
  import { mixer } from "../lib/state/mixer.svelte";
  import { setContentSize } from "../lib/window";
  import InputStrip from "./InputStrip.svelte";
  import BusStrip from "./BusStrip.svelte";

  import { BUS_IDS, MAX_STRIPS, isOutputBus } from "../lib/types";

  let hardware = $derived(mixer.strips.filter((s) => s.kind === "hardware"));
  let virtual = $derived(mixer.strips.filter((s) => s.kind === "virtual"));
  let network = $derived(mixer.strips.filter((s) => s.kind === "vban"));

  let stripsFull = $derived(mixer.strips.length >= MAX_STRIPS);
  let freeA = $derived(BUS_IDS.filter((b) => isOutputBus(b) && !mixer.buses.some((x) => x.id === b)).length);
  let freeB = $derived(BUS_IDS.filter((b) => !isOutputBus(b) && !mixer.buses.some((x) => x.id === b)).length);

  const WINDOW_H = 800;
  let rack: HTMLElement;

  // The window is non-resizable; fit it to the strip content whenever the
  // strip/bus population changes.
  $effect(() => {
    void mixer.strips.length;
    void mixer.buses.length;
    if (!rack) return;
    requestAnimationFrame(() => {
      const groups = Array.from(rack.querySelectorAll<HTMLElement>(".group"));
      if (groups.length === 0) return;
      const width = groups.reduce((sum, g) => sum + g.offsetWidth, 0) + 16 * (groups.length + 1);
      void setContentSize(width, WINDOW_H);
    });
  });
</script>

<main class="rack" bind:this={rack}>
  <section class="group">
    <div class="grouplabel">
      Hardware
      <button
        class="addbtn"
        disabled={stripsFull}
        title={stripsFull ? `At most ${MAX_STRIPS} strips` : "Add a hardware input strip"}
        onclick={() => mixer.addStrip("hardware")}>+</button
      >
    </div>
    <div class="striprow">
      {#each hardware as strip (strip.id)}
        <InputStrip {strip} />
      {/each}
    </div>
  </section>

  <section class="group">
    <div class="grouplabel">
      Virtual
      <button
        class="addbtn"
        disabled={stripsFull}
        title={stripsFull ? `At most ${MAX_STRIPS} strips` : "Add a virtual input (a new playback device)"}
        onclick={() => mixer.addStrip("virtual")}>+</button
      >
    </div>
    <div class="striprow">
      {#each virtual as strip (strip.id)}
        <InputStrip {strip} />
      {/each}
    </div>
  </section>

  {#if network.length > 0}
    <section class="group">
      <div class="grouplabel">Network</div>
      <div class="striprow">
        {#each network as strip (strip.id)}
          <InputStrip {strip} />
        {/each}
      </div>
    </section>
  {/if}

  <section class="group busgroup">
    <div class="grouplabel">
      Buses
      <button
        class="addbtn"
        disabled={freeA === 0}
        title={freeA === 0 ? "All five A buses are in use" : "Add a hardware output bus (A)"}
        onclick={() => mixer.addBus(true)}>+A</button
      >
      <button
        class="addbtn"
        disabled={freeB === 0}
        title={freeB === 0 ? "All three B buses are in use" : "Add a virtual microphone bus (B)"}
        onclick={() => mixer.addBus(false)}>+B</button
      >
    </div>
    <div class="striprow">
      {#each mixer.buses as bus (bus.id)}
        <BusStrip {bus} />
      {/each}
    </div>
  </section>
</main>

<style>
  .rack {
    flex: 1;
    display: flex;
    align-items: stretch;
    gap: 16px;
    padding: 16px;
    background: var(--bg-1);
    /* Only scrolls once the window has hit the screen width. */
    overflow-x: auto;
    overflow-y: hidden;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: none;
    contain: layout paint style;
  }
  .grouplabel {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 16px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-2);
    padding-left: 2px;
  }
  .addbtn {
    height: 16px;
    min-width: 16px;
    padding: 0 4px;
    font: 600 10px var(--mono);
    letter-spacing: 0.04em;
    color: var(--text-3);
    background: var(--bg-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
  }
  .addbtn:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
  }
  .addbtn:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .striprow {
    display: flex;
    align-items: stretch;
    gap: 8px;
    flex: 1;
  }
</style>
