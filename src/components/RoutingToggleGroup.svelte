<script lang="ts">
  import type { BusId } from "../lib/types";

  interface Props {
    /** The buses that exist, in display order. */
    buses: BusId[];
    routes: Partial<Record<BusId, boolean>>;
    ontoggle: (bus: BusId, on: boolean) => void;
  }
  let { buses, routes, ontoggle }: Props = $props();

  // Two per row for the classic four-bus layout; four per row once the
  // mixer grows toward Potato's eight, so the grid never gets taller than two rows.
  let cols = $derived(buses.length <= 4 ? 2 : 4);
</script>

<div class="routes" style:grid-template-columns={`repeat(${cols}, 1fr)`}>
  {#each buses as bus (bus)}
    <button
      class="rbtn"
      class:active={!!routes[bus]}
      aria-pressed={!!routes[bus]}
      onclick={() => ontoggle(bus, !routes[bus])}
    >
      {bus}
    </button>
  {/each}
</div>

<style>
  .routes {
    display: grid;
    gap: 4px;
  }
  .rbtn {
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
  .rbtn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
  .rbtn.active {
    background: var(--accent-glow);
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
