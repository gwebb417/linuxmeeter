<script lang="ts">
  import type { BusId, MacroAction, MacroButton, Switch } from "../lib/types";
  import { FX_KINDS, LABEL_MAX, MAX_MACRO_ACTIONS, MAX_MACROS } from "../lib/types";
  import { mixer } from "../lib/state/mixer.svelte";
  import { ui } from "../lib/state/ui.svelte";
  import { RESERVED_SHORTCUTS, shortcutName } from "../lib/utils/keys";

  // Edits stay local until SAVE; the engine then normalizes the list.
  const copy = <T,>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
  let draft = $state<MacroButton[]>(copy(mixer.macros));
  let sel = $state<number | null>(mixer.macros[0]?.id ?? null);
  let capturing = $state(false);
  let selected = $derived(draft.find((m) => m.id === sel));

  const TYPES: { value: MacroAction["type"]; label: string }[] = [
    { value: "mute", label: "Mute" },
    { value: "solo", label: "Solo" },
    { value: "route", label: "Route" },
    { value: "gain", label: "Fader" },
    { value: "fx_mute", label: "FX return mute" },
    { value: "vban_send", label: "VBAN out" },
    { value: "default_output", label: "Default out" },
  ];
  const SWITCHES: { value: Switch; label: string }[] = [
    { value: "on", label: "on" },
    { value: "off", label: "off" },
    { value: "toggle", label: "toggle" },
  ];

  let strips = $derived(mixer.strips.map((s) => ({ value: `s:${s.id}`, label: s.label })));
  let buses = $derived(mixer.buses.map((b) => ({ value: `b:${b.id}`, label: `${b.id} ${b.label}` })));

  type Tgt = { strip: number } | { bus: BusId };
  const tgtKey = (t: Tgt) => ("strip" in t ? `s:${t.strip}` : `b:${t.bus}`);
  const keyTgt = (k: string): Tgt => (k.startsWith("s:") ? { strip: Number(k.slice(2)) } : { bus: k.slice(2) as BusId });

  function fresh(type: MacroAction["type"]): MacroAction {
    const s = mixer.strips[0]?.id ?? 1;
    const b = mixer.buses[0]?.id ?? "A1";
    switch (type) {
      case "mute":
        return { type, target: { strip: s }, value: "toggle" };
      case "solo":
        return { type, strip: s, value: "toggle" };
      case "route":
        return { type, strip: s, bus: b, value: "toggle" };
      case "gain":
        return { type, target: { strip: s }, db: 0 };
      case "fx_mute":
        return { type, fx: "reverb", value: "toggle" };
      case "vban_send":
        return { type, bus: b, value: "toggle" };
      case "default_output":
        return { type, value: "toggle" };
    }
  }

  function add() {
    if (draft.length >= MAX_MACROS) return;
    const id = Math.max(0, ...draft.map((m) => m.id)) + 1;
    draft.push({ id, label: `Macro ${id}`, mode: "toggle", on: [], off: [], key: null, active: false });
    sel = id;
  }
  function removeSelected() {
    const i = draft.findIndex((m) => m.id === sel);
    if (i < 0) return;
    draft.splice(i, 1);
    sel = draft[Math.min(i, draft.length - 1)]?.id ?? null;
  }
  function addAction(list: "on" | "off") {
    if (selected && selected[list].length < MAX_MACRO_ACTIONS) selected[list].push(fresh("mute"));
  }

  let keyClash = $derived(
    selected?.key ? draft.find((m) => m.id !== selected.id && m.key === selected.key)?.label : undefined,
  );

  function onkeydown(e: KeyboardEvent) {
    if (capturing && selected) {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        capturing = false;
        return;
      }
      if (e.key === "Backspace" || e.key === "Delete") {
        selected.key = null;
        capturing = false;
        return;
      }
      const name = shortcutName(e);
      if (!name || RESERVED_SHORTCUTS.has(name)) return;
      selected.key = name;
      capturing = false;
      return;
    }
    if (e.key === "Escape") close();
  }

  function save() {
    mixer.setMacros(copy(draft));
    close();
  }
  function close() {
    ui.macroEditor = false;
  }
</script>

<svelte:window {onkeydown} />

{#snippet actionList(list: "on" | "off", title: string, hint: string)}
  {#if selected}
    <div class="alist">
      <div class="ahead">
        <span class="secname">{title}</span>
        <span class="hint">{hint}</span>
        <button class="small" disabled={selected[list].length >= MAX_MACRO_ACTIONS} onclick={() => addAction(list)}>+ ACTION</button>
      </div>
      {#each selected[list] as a, i (i)}
        <div class="arow">
          <select
            value={a.type}
            aria-label="Action"
            onchange={(e) => (selected[list][i] = fresh((e.currentTarget as HTMLSelectElement).value as MacroAction["type"]))}
          >
            {#each TYPES as t}<option value={t.value}>{t.label}</option>{/each}
          </select>

          {#if a.type === "mute" || a.type === "gain"}
            <select value={tgtKey(a.target)} aria-label="Strip or bus" onchange={(e) => (a.target = keyTgt((e.currentTarget as HTMLSelectElement).value))}>
              <optgroup label="Strips">{#each strips as o}<option value={o.value}>{o.label}</option>{/each}</optgroup>
              <optgroup label="Buses">{#each buses as o}<option value={o.value}>{o.label}</option>{/each}</optgroup>
            </select>
          {:else if a.type === "solo" || a.type === "route"}
            <select value={`s:${a.strip}`} aria-label="Strip" onchange={(e) => (a.strip = Number((e.currentTarget as HTMLSelectElement).value.slice(2)))}>
              {#each strips as o}<option value={o.value}>{o.label}</option>{/each}
            </select>
          {/if}
          {#if a.type === "route" || a.type === "vban_send"}
            <select value={a.bus} aria-label="Bus" onchange={(e) => (a.bus = (e.currentTarget as HTMLSelectElement).value as BusId)}>
              {#each mixer.buses as b}<option value={b.id}>{b.id} {b.label}</option>{/each}
            </select>
          {/if}
          {#if a.type === "fx_mute"}
            <select value={a.fx} aria-label="Effect" onchange={(e) => (a.fx = (e.currentTarget as HTMLSelectElement).value as typeof a.fx)}>
              {#each FX_KINDS as k}<option value={k}>{k}</option>{/each}
            </select>
          {/if}

          {#if a.type === "gain"}
            <input
              class="db"
              type="number"
              min="-80"
              max="12"
              step="0.5"
              value={a.db}
              aria-label="Fader dB"
              onchange={(e) => (a.db = Number((e.currentTarget as HTMLInputElement).value))}
            /><span class="unit">dB</span>
          {:else}
            <select value={a.value} aria-label="Value" onchange={(e) => (a.value = (e.currentTarget as HTMLSelectElement).value as Switch)}>
              {#each SWITCHES as o}<option value={o.value}>{o.label}</option>{/each}
            </select>
          {/if}
          <button class="small x" title="Remove action" aria-label="Remove action" onclick={() => selected[list].splice(i, 1)}>✕</button>
        </div>
      {:else}
        <div class="none">nothing</div>
      {/each}
    </div>
  {/if}
{/snippet}

<!-- scrim click closes; Esc handled on window -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" onclick={(e) => e.target === e.currentTarget && close()}>
  <div class="panel" role="dialog" aria-label="Macro buttons">
    <header>
      <div class="title">MACROS <span class="sub">buttons that change several things at once</span></div>
      <button class="closebtn" title="Close without saving" onclick={close}>
        <svg width="11" height="11" viewBox="0 0 12 12"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.25" /></svg>
      </button>
    </header>

    <div class="body">
      <div class="list">
        {#each draft as m (m.id)}
          <button class="item" class:sel={m.id === sel} onclick={() => (sel = m.id)}>
            <span>{m.label}</span>
            {#if m.key}<span class="pkey">{m.key}</span>{/if}
          </button>
        {/each}
        <button class="small" disabled={draft.length >= MAX_MACROS} onclick={add}>+ NEW</button>
      </div>

      <div class="form">
        {#if selected}
          <div class="frow">
            <input maxlength={LABEL_MAX} bind:value={selected.label} aria-label="Button name" />
            <div class="seg" role="group" aria-label="Mode">
              <button class:active={selected.mode === "toggle"} onclick={() => (selected.mode = "toggle")} title="Each press switches it on or off">TOGGLE</button>
              <button class:active={selected.mode === "push"} onclick={() => (selected.mode = "push")} title="On while held, off on release">PUSH</button>
            </div>
            <button class="small" class:capturing onclick={() => (capturing = !capturing)} title="Shortcut while the window has focus">
              {capturing ? "press a key… (Del clears)" : selected.key ? `KEY ${selected.key}` : "SET KEY"}
            </button>
          </div>
          {#if keyClash}<div class="warn">{selected.key} is also used by "{keyClash}".</div>{/if}
          {@render actionList(
            "on",
            selected.mode === "push" ? "ON PRESS" : "TURN ON",
            selected.mode === "push" ? "while held" : "first click",
          )}
          {@render actionList(
            "off",
            selected.mode === "push" ? "ON RELEASE" : "TURN OFF",
            selected.mode === "push" ? "when let go" : "second click",
          )}
          <div class="frow end">
            <button class="small danger" onclick={removeSelected}>DELETE BUTTON</button>
          </div>
        {:else}
          <div class="none">Add a button with + NEW.</div>
        {/if}
      </div>
    </div>

    <footer>
      <button class="small" onclick={close}>CANCEL</button>
      <button class="small primary" onclick={save}>SAVE</button>
    </footer>
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
    width: 680px;
    max-height: calc(100% - 32px);
    background: var(--bg-2);
    border: 1px solid var(--border-1);
    border-radius: 6px;
    box-shadow: inset 0 1px 0 var(--hl-top), 0 8px 24px rgba(0, 0, 0, 0.6);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  header,
  footer {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  footer {
    justify-content: flex-end;
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
  .body {
    display: flex;
    gap: 12px;
    min-height: 0;
    overflow-y: auto;
  }
  .list {
    width: 150px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    font-size: 11px;
    color: var(--text-2);
    background: var(--bg-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
    text-align: left;
  }
  .item span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item.sel {
    border-color: var(--accent);
    color: var(--text-1);
  }
  .pkey {
    font: 500 9px var(--mono);
    color: var(--text-3);
  }
  .form {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .frow {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .frow.end {
    justify-content: flex-end;
  }
  .frow input {
    flex: 1;
  }
  input,
  select {
    min-width: 0;
    height: 22px;
    padding: 0 6px;
    background: var(--bg-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
    box-shadow: inset 0 1px 0 var(--hl-top);
    font-size: 11px;
    color: var(--text-1);
  }
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
  }
  .seg {
    display: flex;
  }
  .seg button {
    height: 22px;
    padding: 0 8px;
    font: 500 10px var(--mono);
    letter-spacing: 0.06em;
    background: var(--bg-3);
    color: var(--text-3);
    border: 1px solid var(--border-0);
  }
  .seg button:first-child {
    border-radius: 2px 0 0 2px;
  }
  .seg button:last-child {
    border-radius: 0 2px 2px 0;
    border-left: none;
  }
  .seg button.active {
    background: var(--accent-glow);
    color: var(--accent);
    border-color: var(--accent);
  }
  .alist {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .ahead {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .ahead .small {
    margin-left: auto;
  }
  .secname {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-2);
  }
  .hint,
  .none {
    font-size: 10px;
    color: var(--text-3);
  }
  .arow {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .arow select {
    flex: 1;
  }
  .db {
    width: 64px;
    font-family: var(--mono);
  }
  .unit {
    font: 500 10px var(--mono);
    color: var(--text-3);
  }
  .warn {
    font-size: 10px;
    color: var(--mute);
  }
  .small {
    height: 22px;
    padding: 0 8px;
    font: 500 10px var(--mono);
    letter-spacing: 0.06em;
    background: var(--bg-3);
    color: var(--text-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
    white-space: nowrap;
  }
  .small:hover:not(:disabled) {
    color: var(--text-1);
    background: var(--bg-4);
  }
  .small:disabled {
    opacity: 0.35;
  }
  .small.x {
    width: 22px;
    padding: 0;
  }
  .small.capturing {
    color: var(--accent);
    border-color: var(--accent);
  }
  .small.primary {
    background: var(--accent-glow);
    color: var(--accent);
    border-color: var(--accent);
  }
  .small.danger:hover {
    background: var(--mute);
    border-color: var(--mute);
    color: #fff;
  }
</style>
