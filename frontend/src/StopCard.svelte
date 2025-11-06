<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import RouteArrivals from './RouteArrivals.svelte';
  import type { StopArrival } from './lib/types';

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
    {#if Object.values(stop.arrivals).length === 0}
      <p>No arrivals available at this time.</p>
    {:else}
      {#each Object.entries(stop.arrivals) as [type, routes]}
        {@const sortedRoutes = (browsing && selectedRoute)
          ? Object.entries(routes).sort(([a], [b]) => {
              if (a === selectedRoute) return -1;
              if (b === selectedRoute) return 1;
              return 0;
            })
          : Object.entries(routes)}
        {#each sortedRoutes as [route, arrivals]}
          {#if !hiddenRoutes.includes(route)}
            <RouteArrivals {type} {route} {arrivals} />
          {/if}
        {/each}
      {/each}
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
  }
</style>