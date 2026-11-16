<script lang="ts">
  import DeviceSelect from "./DeviceSelect.svelte";
  import { LABEL_MAX } from "../lib/types";

  interface Option {
    value: string;
    label: string;
  }
  interface Props {
    label: string;
    busBadge?: string;
    deviceValue: string | null;
    deviceDisplay: string;
    deviceOptions: Option[];
    ondevicechange?: (value: string) => void;
    deviceDisabled?: boolean;
    /** Double-click the name to rename. */
    onrename?: (label: string) => void;
    /** Shows a remove button (two clicks: arm, then confirm). */
    onremove?: () => void;
  }
  let {
    label,
    busBadge,
    deviceValue,
    deviceDisplay,
    deviceOptions,
    ondevicechange,
    deviceDisabled,
    onrename,
    onremove,
  }: Props = $props();

  let editing = $state(false);
  let draft = $state("");
  let armed = $state(false);
  let armTimer: ReturnType<typeof setTimeout> | undefined;

  function startEdit() {
    if (!onrename) return;
    draft = label;
    editing = true;
  }

  function commit() {
    if (!editing) return;
    editing = false;
    const next = draft.trim();
    if (next && next !== label) onrename?.(next);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Enter") commit();
    else if (e.key === "Escape") editing = false;
  }

  function focus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function remove() {
    if (!armed) {
      armed = true;
      clearTimeout(armTimer);
      armTimer = setTimeout(() => (armed = false), 3000);
      return;
    }
    clearTimeout(armTimer);
    armed = false;
    onremove?.();
  }
</script>

<div class="head">
  <div class="nrow">
    {#if busBadge}<span class="busid">{busBadge}</span>{/if}
    {#if editing}
      <input
        class="nameinput"
        maxlength={LABEL_MAX}
        bind:value={draft}
        onblur={commit}
        {onkeydown}
        use:focus
        aria-label="Name"
      />
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="name" class:renamable={!!onrename} title={onrename ? "Double-click to rename" : undefined} ondblclick={startEdit}>
        {label}
      </div>
    {/if}
    {#if onremove && !editing}
      <button
        class="rm"
        class:armed
        title={armed ? "Click again to remove" : "Remove"}
        aria-label={armed ? `Confirm removing ${label}` : `Remove ${label}`}
        onclick={remove}
      >
        {#if armed}DEL?{:else}
          <svg width="8" height="8" viewBox="0 0 12 12"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.5" /></svg>
        {/if}
      </button>
    {/if}
  </div>
  <DeviceSelect
    value={deviceValue}
    display={deviceDisplay}
    options={deviceOptions}
    onchange={ondevicechange}
    disabled={deviceDisabled}
  />
</div>

<style>
  .head {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .nrow {
    position: relative;
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    height: 17px;
  }
  .busid {
    font: 600 10px var(--mono);
    letter-spacing: 0.08em;
    color: var(--accent);
  }
  .name {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name.renamable {
    cursor: text;
  }
  .nameinput {
    flex: 1;
    min-width: 0;
    height: 17px;
    padding: 0 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-1);
    background: var(--bg-3);
    border: 1px solid var(--accent);
    border-radius: 2px;
    outline: none;
  }
  /* Floats over the end of the name so it costs no width until shown. */
  .rm {
    position: absolute;
    right: 0;
    top: 1px;
    height: 15px;
    min-width: 15px;
    padding: 0 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    font: 600 8px var(--mono);
    letter-spacing: 0.04em;
    color: var(--text-3);
    background: var(--bg-2);
    border-radius: 2px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.12s;
  }
  .head:hover .rm,
  .rm:focus-visible,
  .rm.armed {
    opacity: 1;
    pointer-events: auto;
  }
  .rm:hover {
    color: var(--text-1);
    background: var(--bg-4);
  }
  .rm.armed {
    color: #fff;
    background: var(--mute);
  }
</style>
