<script lang="ts">
  import { onMount } from "svelte";
  import { mixer } from "./lib/state/mixer.svelte";
  import { ui } from "./lib/state/ui.svelte";
  import TopBar from "./components/TopBar.svelte";
  import StripRack from "./components/StripRack.svelte";
  import EqPanel from "./components/EqPanel.svelte";
  import DynPanel from "./components/DynPanel.svelte";
  import VbanPanel from "./components/VbanPanel.svelte";
  import FxPanel from "./components/FxPanel.svelte";
  import { BUS_EQ_BANDS, STRIP_EQ_BANDS } from "./lib/types";

  onMount(() => {
    void mixer.init();
  });

  let eqStrip = $derived(ui.eqStrip !== null ? mixer.strip(ui.eqStrip) : undefined);
  let eqBus = $derived(ui.eqBus !== null ? mixer.bus(ui.eqBus) : undefined);
  let dynStrip = $derived(ui.dynStrip !== null ? mixer.strip(ui.dynStrip) : undefined);
</script>

<div class="app">
  <TopBar />
  {#if mixer.ready}
    <StripRack />
  {:else}
    <main class="loading">connecting…</main>
  {/if}
</div>
{#if eqStrip}
  {@const s = eqStrip}
  <EqPanel
    title={s.label}
    eq={s.eq}
    editable={4}
    resetBands={STRIP_EQ_BANDS}
    onchange={(eq) => mixer.setEqParams(s.id, eq)}
    onclose={() => (ui.eqStrip = null)}
  />
{:else if eqBus}
  {@const b = eqBus}
  <EqPanel
    title={`${b.id} ${b.label}`}
    eq={b.eq}
    editable={8}
    resetBands={BUS_EQ_BANDS}
    onchange={(eq) => mixer.setBusEqParams(b.id, eq)}
    onclose={() => (ui.eqBus = null)}
  />
{/if}
{#if dynStrip}
  <DynPanel strip={dynStrip} />
{/if}
{#if ui.vbanOpen && mixer.ready}
  <VbanPanel />
{/if}
{#if ui.fxOpen && mixer.ready}
  <FxPanel />
{/if}
{#if mixer.notice}
  <div class="notice" role="alert">{mixer.notice}</div>
{/if}

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .notice {
    position: fixed;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    z-index: 50;
    max-width: 560px;
    padding: 8px 12px;
    background: var(--bg-3);
    border: 1px solid var(--mute);
    border-radius: 4px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.6);
    font-size: 11px;
    color: var(--text-1);
  }
  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-1);
    color: var(--text-3);
    font: 500 11px var(--mono);
    letter-spacing: 0.08em;
  }
</style>
