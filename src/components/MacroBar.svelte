<script lang="ts">
  import { mixer } from "../lib/state/mixer.svelte";
  import { ui } from "../lib/state/ui.svelte";

  // Which pads this pointer is holding, so a drag off the pad still releases it.
  const held = new Set<number>();

  function down(e: PointerEvent, id: number) {
    if (e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    held.add(id);
    mixer.triggerMacro(id, true);
  }
  function up(id: number) {
    if (!held.delete(id)) return;
    mixer.triggerMacro(id, false);
  }
  function key(e: KeyboardEvent, id: number, pressed: boolean) {
    if (e.key !== " " && e.key !== "Enter") return;
    e.preventDefault();
    if (pressed && e.repeat) return;
    mixer.triggerMacro(id, pressed);
  }
</script>

<div class="bar" role="toolbar" aria-label="Macro buttons">
  <span class="barlabel">MACROS</span>
  {#each mixer.macros as m (m.id)}
    <button
      class="pad"
      class:active={m.active}
      class:push={m.mode === "push"}
      aria-pressed={m.active}
      title={`${m.label}${m.key ? ` (${m.key})` : ""} · ${m.mode === "push" ? "hold" : "click to toggle"}`}
      onpointerdown={(e) => down(e, m.id)}
      onpointerup={() => up(m.id)}
      onpointercancel={() => up(m.id)}
      onkeydown={(e) => key(e, m.id, true)}
      onkeyup={(e) => key(e, m.id, false)}
    >
      <span class="plabel">{m.label}</span>
      {#if m.key}<span class="pkey">{m.key}</span>{/if}
    </button>
  {:else}
    <span class="empty">No macro buttons yet.</span>
  {/each}
  <button class="edit" title="Add and edit macro buttons" onclick={() => (ui.macroEditor = true)}>
    {mixer.macros.length ? "EDIT" : "+ ADD"}
  </button>
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 16px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border-0);
    overflow-x: auto;
    flex: none;
  }
  .barlabel {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-3);
    margin-right: 4px;
  }
  .pad {
    flex: none;
    min-width: 72px;
    max-width: 140px;
    height: 34px;
    padding: 0 10px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    background: var(--bg-3);
    color: var(--text-2);
    border: 1px solid var(--border-0);
    border-radius: 3px;
    box-shadow: inset 0 1px 0 var(--hl-top);
    touch-action: none;
    user-select: none;
  }
  .pad:hover {
    background: var(--bg-4);
    color: var(--text-1);
  }
  .pad.active {
    background: var(--accent-glow);
    color: var(--accent);
    border-color: var(--accent);
  }
  .pad.push {
    border-style: dashed;
  }
  .pad.push.active {
    border-style: solid;
  }
  .plabel {
    font-size: 11px;
    font-weight: 600;
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pkey {
    font: 500 8px var(--mono);
    color: var(--text-3);
  }
  .empty {
    font-size: 11px;
    color: var(--text-3);
  }
  .edit {
    flex: none;
    margin-left: auto;
    height: 22px;
    padding: 0 10px;
    font: 500 10px var(--mono);
    letter-spacing: 0.06em;
    background: var(--bg-3);
    color: var(--text-3);
    border: 1px solid var(--border-0);
    border-radius: 2px;
  }
  .edit:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
