<script lang="ts">
  import type { BusState, StripState, VbanRecv, VbanSend } from "../lib/types";
  import { VBAN_DEFAULT_PORT, VBAN_RATES, VBAN_STREAM_NAME_MAX } from "../lib/types";
  import { mixer } from "../lib/state/mixer.svelte";
  import { ui } from "../lib/state/ui.svelte";

  // Fields commit on `change` (blur / Enter). The store is optimistic; if the
  // engine rejects a value it resends state and the input snaps back.

  function setSend(bus: BusState, patch: Partial<VbanSend>) {
    mixer.setBusVban(bus.id, { ...bus.vban, ...patch });
  }

  function setRecv(strip: StripState, patch: Partial<VbanRecv>, label = strip.label) {
    if (!strip.vban) return;
    mixer.setStripVban(strip.id, label, { ...strip.vban, ...patch });
  }

  const port = (e: Event) => Number((e.currentTarget as HTMLInputElement).value);
  const text = (e: Event) => (e.currentTarget as HTMLInputElement).value;

  let inputs = $derived(mixer.strips.filter((s) => s.kind === "vban"));

  let newLabel = $state("");
  let draft = $state<VbanRecv>({ streamName: "Stream1", port: VBAN_DEFAULT_PORT, senderIp: "", latencyMs: 100 });

  function add() {
    mixer.addVbanStrip(newLabel, { ...draft });
    newLabel = "";
  }

  function close() {
    ui.vbanOpen = false;
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") close();
  }
</script>

<svelte:window {onkeydown} />

<!-- scrim click closes; Esc handled on window -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && close()}>
  <div class="panel" role="dialog" aria-label="VBAN">
    <header>
      <div class="title">VBAN <span class="sub">network audio</span></div>
      <button class="closebtn" title="Close" onclick={close}>
        <svg width="11" height="11" viewBox="0 0 12 12"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.25" /></svg>
      </button>
    </header>

    <section>
      <div class="sechead">
        <span class="secname">OUTGOING</span>
        <span class="hint">post-limiter bus mix → UDP</span>
      </div>
      <div class="grid send">
        <span class="colhead"></span>
        <span class="colhead">BUS</span>
        <span class="colhead">STREAM</span>
        <span class="colhead">DESTINATION IP</span>
        <span class="colhead">PORT</span>
        <span class="colhead">RATE</span>
        {#each mixer.buses as bus (bus.id)}
          <button
            class="enbtn"
            class:active={bus.vban.enabled}
            aria-pressed={bus.vban.enabled}
            title={bus.vban.enabled ? "Stop sending" : "Start sending"}
            onclick={() => setSend(bus, { enabled: !bus.vban.enabled })}
          >
            {bus.vban.enabled ? "ON" : "OFF"}
          </button>
          <span class="busname"><span class="busid">{bus.id}</span>{bus.label}</span>
          <input
            maxlength={VBAN_STREAM_NAME_MAX}
            value={bus.vban.streamName}
            onchange={(e) => setSend(bus, { streamName: text(e) })}
            aria-label="{bus.id} stream name"
          />
          <input
            class="mono"
            value={bus.vban.host}
            placeholder="192.168.1.20"
            onchange={(e) => setSend(bus, { host: text(e) })}
            aria-label="{bus.id} destination IP"
          />
          <input
            class="mono"
            type="number"
            min="1"
            max="65535"
            value={bus.vban.port}
            onchange={(e) => setSend(bus, { port: port(e) })}
            aria-label="{bus.id} port"
          />
          <select
            value={bus.vban.rate}
            onchange={(e) => setSend(bus, { rate: Number((e.currentTarget as HTMLSelectElement).value) })}
            aria-label="{bus.id} sample rate"
          >
            {#each VBAN_RATES as r}
              <option value={r}>{r / 1000} kHz</option>
            {/each}
          </select>
        {/each}
      </div>
    </section>

    <div class="sep"></div>

    <section>
      <div class="sechead">
        <span class="secname">INCOMING</span>
        <span class="hint">each stream becomes a strip in the Network group</span>
      </div>
      <div class="grid recv">
        <span class="colhead">STRIP</span>
        <span class="colhead">STREAM</span>
        <span class="colhead">PORT</span>
        <span class="colhead">FROM IP</span>
        <span class="colhead">BUFFER</span>
        <span class="colhead"></span>
        {#each inputs as strip (strip.id)}
          {#if strip.vban}
            <span class="stripname">
              <span class="dot" class:live={strip.online} title={strip.online ? "receiving" : "no signal"}></span>
              <input
                value={strip.label}
                onchange={(e) => setRecv(strip, {}, text(e))}
                aria-label="strip label"
              />
            </span>
            <input
              maxlength={VBAN_STREAM_NAME_MAX}
              value={strip.vban.streamName}
              onchange={(e) => setRecv(strip, { streamName: text(e) })}
              aria-label="{strip.label} stream name"
            />
            <input
              class="mono"
              type="number"
              min="1"
              max="65535"
              value={strip.vban.port}
              onchange={(e) => setRecv(strip, { port: port(e) })}
              aria-label="{strip.label} port"
            />
            <input
              class="mono"
              value={strip.vban.senderIp}
              placeholder="any"
              onchange={(e) => setRecv(strip, { senderIp: text(e) })}
              aria-label="{strip.label} sender IP"
            />
            <input
              class="mono"
              type="number"
              min="10"
              max="2000"
              step="10"
              value={strip.vban.latencyMs}
              onchange={(e) => setRecv(strip, { latencyMs: port(e) })}
              aria-label="{strip.label} buffer in ms"
              title="Jitter buffer, ms"
            />
            <button class="enbtn danger" title="Remove this strip" onclick={() => mixer.removeStrip(strip.id)}>
              DEL
            </button>
          {/if}
        {/each}

        <input bind:value={newLabel} placeholder="new strip" aria-label="new strip label" />
        <input maxlength={VBAN_STREAM_NAME_MAX} bind:value={draft.streamName} aria-label="new stream name" />
        <input class="mono" type="number" min="1" max="65535" bind:value={draft.port} aria-label="new port" />
        <input class="mono" bind:value={draft.senderIp} placeholder="any" aria-label="new sender IP" />
        <input
          class="mono"
          type="number"
          min="10"
          max="2000"
          step="10"
          bind:value={draft.latencyMs}
          aria-label="new buffer in ms"
          title="Jitter buffer, ms"
        />
        <button class="enbtn active" title="Add a VBAN input strip" onclick={add}>ADD</button>
      </div>
      <p class="foot">
        Stream names are matched exactly (max 16 ASCII chars). Filtering by stream name needs PipeWire 1.4 or
        newer; on older versions each port carries one stream. Addresses must be IP literals.
      </p>
    </section>
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
    width: 640px;
    max-height: calc(100% - 32px);
    overflow-y: auto;
    background: var(--bg-2);
    border: 1px solid var(--border-1);
    border-radius: 6px;
    box-shadow: inset 0 1px 0 var(--hl-top), 0 8px 24px rgba(0, 0, 0, 0.6);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
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
  .sep {
    height: 1px;
    background: var(--border-0);
    margin: 0 -12px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .sechead {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }
  .secname {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-2);
  }
  .hint,
  .foot {
    font-size: 10px;
    color: var(--text-3);
  }
  .foot {
    margin: 2px 0 0;
    line-height: 1.4;
  }
  .grid {
    display: grid;
    gap: 5px 6px;
    align-items: center;
  }
  .grid.send {
    grid-template-columns: 44px 1.2fr 1.2fr 1.4fr 70px 80px;
  }
  .grid.recv {
    grid-template-columns: 1.3fr 1.2fr 70px 1.2fr 64px 44px;
  }
  .colhead {
    font: 500 9px var(--mono);
    letter-spacing: 0.06em;
    color: var(--text-3);
  }
  .busname,
  .stripname {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 11px;
    color: var(--text-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .busid {
    font: 600 10px var(--mono);
    letter-spacing: 0.08em;
    color: var(--accent);
  }
  .dot {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-3);
  }
  .dot.live {
    background: var(--meter-green);
  }
  input,
  select {
    min-width: 0;
    width: 100%;
    height: 22px;
    padding: 0 6px;
    background: var(--bg-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
    box-shadow: inset 0 1px 0 var(--hl-top);
    font-size: 11px;
    color: var(--text-1);
  }
  input.mono {
    font-family: var(--mono);
  }
  input::placeholder {
    color: var(--text-3);
  }
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
  }
  .enbtn {
    height: 22px;
    padding: 0 6px;
    font: 500 10px var(--mono);
    letter-spacing: 0.06em;
    background: var(--bg-3);
    color: var(--text-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
    box-shadow: inset 0 1px 0 var(--hl-top);
  }
  .enbtn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
  .enbtn.active {
    background: var(--accent-glow);
    color: var(--accent);
    border-color: var(--accent);
  }
  .enbtn.danger:hover {
    background: var(--mute);
    border-color: var(--mute);
    color: #fff;
  }
</style>
