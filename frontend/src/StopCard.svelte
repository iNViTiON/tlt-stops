<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { ArrivalEntry, StopArrival } from './lib/types';
  import { hiddenKey, isHiddenRoute } from './lib/hiddenRoutes';
  import { currentTime } from './lib/stores';

  export let stop: StopArrival;
  export let hiddenRoutes: string[] = [];
  export let selectedRoute: string | undefined = undefined;
  export let selectedType: string | undefined = undefined;
  export let browsing: boolean = false;

  const dispatch = createEventDispatcher();

  let showFilters = false;

  function toggleRoute(type: string, route: string) {
    dispatch('toggleHidden', hiddenKey(type, route));
  }

  function getIcon(type: string): string {
    switch (type) {
      case 'bus': return '🚌';
      case 'commercialbus':
      case 'regionalbus': return '🚍';
      case 'train': return '🚆';
      case 'tram': return '🚋';
      case 'trol':
      case 'trolleybus': return '🚎';
      default: return '❓';
    }
  }

  // One formatter for the component instead of one per cell per second.
  const timeFormat = new Intl.DateTimeFormat('en-GB', {
    hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false
  });

  type Row = { route: string; type: string; slots: (ArrivalEntry | null)[] };
  type Grid = { times: number[]; labels: string[]; rows: Row[] };

  // Columns are unique departure instants, ordered numerically. Sorting the
  // formatted "HH:MM:SS" text instead would put 00:10 tomorrow ahead of 23:55
  // tonight.
  function buildGrid(
    stop: StopArrival,
    hiddenRoutes: string[],
    browsing: boolean,
    selectedRoute: string | undefined,
    selectedType: string | undefined
  ): Grid {
    const column = new Map<number, number>();
    const visible: Array<{ route: string; type: string; arrivals: ArrivalEntry[] }> = [];

    for (const [type, routes] of Object.entries(stop.arrivals)) {
      for (const [route, arrivals] of Object.entries(routes)) {
        if (!browsing && isHiddenRoute(hiddenRoutes, type, route)) continue;
        visible.push({ route, type, arrivals });
        for (const arrival of arrivals) column.set(arrival.time, 0);
      }
    }

    const times = [...column.keys()].sort((a, b) => a - b);
    times.forEach((time, index) => column.set(time, index));

    if (browsing && selectedRoute) {
      const pinned = visible.findIndex(
        row => row.route === selectedRoute && (!selectedType || row.type === selectedType)
      );
      if (pinned > 0) visible.unshift(visible.splice(pinned, 1)[0]);
    }

    const rows = visible.map(({ route, type, arrivals }) => {
      const slots: (ArrivalEntry | null)[] = new Array(times.length).fill(null);
      for (const arrival of arrivals) slots[column.get(arrival.time)!] = arrival;
      return { route, type, slots };
    });

    return { times, labels: times.map(time => timeFormat.format(time)), rows };
  }

  // Rebuilt only when the data or the filters change.
  $: grid = buildGrid(stop, hiddenRoutes, browsing, selectedRoute, selectedType);

  // Ticks every second, but a countdown depends only on the column instant,
  // so this is one pass over columns rather than over every cell.
  $: countdowns = grid.times.map(time => {
    const diff = Math.floor((time - $currentTime) / 1000);
    if (diff <= 0) return 'Now';
    const minutes = Math.floor(diff / 60);
    const seconds = diff % 60;
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  });
</script>

<div class="stop-card">
  <div class="stop-header">
    <h3>{stop.name}</h3>
    {#if !browsing}
      <button class="remove-btn" onclick={() => dispatch('remove')}>✕</button>
      <button class="filter-btn" onclick={() => showFilters = !showFilters}>⚙️</button>
    {/if}
  </div>

  {#if showFilters && !browsing}
    <div class="filters">
      <!-- Only rendered when !browsing, so no selected route to pin here. -->
      {#each Object.entries(stop.arrivals) as [type, routes]}
        {#each Object.entries(routes) as [route]}
          <label>
            <input
              type="checkbox"
              checked={!isHiddenRoute(hiddenRoutes, type, route)}
              onchange={() => toggleRoute(type, route)}
            />
            {type} {route}
          </label>
        {/each}
      {/each}
    </div>
  {/if}

  <div class="arrivals">
    {#if grid.times.length === 0}
      <p>No arrivals available at this time.</p>
    {:else}
      <div class="times-grid">
        {#each grid.rows as row}
          <div class="route-row">
            <span class="route-icon">{getIcon(row.type)}</span>
            <span class="route-number">{row.route}</span>
            {#each row.slots as arrival, column}
              <span class="time-cell">
                {#if arrival}
                  {countdowns[column]}{#if arrival.isLowEntry}♿{/if}
                  <!-- pads cells without the ♿ glyph so columns stay aligned -->
                  <br>{grid.labels[column]}{#if !arrival.isLowEntry}{' '}{/if}
                {:else}
                  ———————
                {/if}
              </span>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .stop-card {
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 1rem;
    background: var(--card-bg);
  }

  .stop-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .remove-btn, .filter-btn {
    background: none;
    border: none;
    font-size: 1.2rem;
    cursor: pointer;
  }

  .filters {
    margin-top: 0.5rem;
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
  }

  .filters label {
    display: block;
    margin-bottom: 0.5rem;
  }

  .arrivals {
    margin-top: 1rem;
    overflow-x: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--border-color) transparent;
  }

  .times-grid {
    display: flex;
    flex-direction: column;
    min-width: fit-content;
  }

  .route-row {
    display: flex;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .route-icon {
    font-size: 1.5rem;
    margin-right: 0.5rem;
  }

  .route-number {
    min-width: 3rem;
    font-weight: bold;
    margin-right: 1rem;
  }

  .time-cell {
    min-width: 4rem;
    text-align: center;
    font-family: monospace;
    margin-right: -0.12rem;
    padding-right: 0.12rem;
  }
</style>