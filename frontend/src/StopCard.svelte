<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { StopArrival } from './lib/types';
  import { currentTime } from './lib/stores';

  export let stop: StopArrival;
  export let hiddenRoutes: string[] = [];
  export let selectedRoute: string | undefined = undefined;
  export let browsing: boolean = false;

  const dispatch = createEventDispatcher();

  let showFilters = false;

  function getVisibleRoutes() {
    const allRoutes = Object.entries(stop.arrivals).flatMap(([type, routes]) =>
      Object.entries(routes).map(([route]) => `${type}-${route}`)
    );
    return allRoutes.filter(route => !hiddenRoutes.includes(route.split('-')[1]));
  }

  function toggleRoute(route: string) {
    dispatch('toggleHidden', route);
  }

  function formatTime(timestamp: number): string {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
  }

  $: allArrivals = Object.entries(stop.arrivals).flatMap(([type, routes]) =>
    Object.entries(routes).flatMap(([route, arrivals]) =>
      arrivals.map(arrival => ({
        route,
        type,
        timestamp: arrival.time,
        time: formatTime(arrival.time),
        isLowEntry: arrival.isLowEntry
      }))
    )
  );

  $: countdowns = allArrivals.map(a => {
    const now = $currentTime;
    const diff = Math.floor((a.timestamp - now) / 1000);
    if (diff <= 0) return 'Now';
    const minutes = Math.floor(diff / 60);
    const seconds = diff % 60;
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  });

  $: allTimes = [...new Set(allArrivals.map(a => a.time))].sort();

  $: visibleRoutes = Object.entries(stop.arrivals).flatMap(([type, routes]) =>
    Object.entries(routes).filter(([route]) => !hiddenRoutes.includes(route)).map(([route]) => route)
  ).sort((a, b) => {
    if (browsing && selectedRoute) {
      if (a === selectedRoute) return -1;
      if (b === selectedRoute) return 1;
    }
    return 0;
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
      {#each Object.entries(stop.arrivals) as [type, routes]}
        {@const sortedRoutes = (browsing && selectedRoute)
          ? Object.entries(routes).sort(([a], [b]) => {
              if (a === selectedRoute) return -1;
              if (b === selectedRoute) return 1;
              return 0;
            })
          : Object.entries(routes)}
        {#each sortedRoutes as [route]}
          <label>
            <input
              type="checkbox"
              checked={!hiddenRoutes.includes(route)}
              onchange={() => toggleRoute(route)}
            />
            {type} {route}
          </label>
        {/each}
      {/each}
    </div>
  {/if}

  <div class="arrivals">
    {#if allTimes.length === 0}
      <p>No arrivals available at this time.</p>
    {:else}
      <div class="times-grid">
        {#each visibleRoutes as route}
          {@const routeArrivals = allArrivals.filter(a => a.route === route)}
          {@const routeType = routeArrivals[0]?.type || ''}
          <div class="route-row">
            <span class="route-number">{route}</span>
            {#each allTimes as time}
              {@const arrival = routeArrivals.find(a => a.time === time)}
              <span class="time-cell">
                {#if arrival}
                  {@const index = allArrivals.indexOf(arrival)}
                  {countdowns[index]}{#if arrival.isLowEntry}♿{/if}
                  {time}{#if !arrival.isLowEntry} {/if}
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