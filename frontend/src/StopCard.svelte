<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import RouteArrivals from './RouteArrivals.svelte';
  import type { StopArrival } from './lib/types';

  export let stop: StopArrival;
  export let hiddenRoutes: string[] = [];

  const dispatch = createEventDispatcher();

  let showFilters = false;

  function getVisibleRoutes() {
    const allRoutes = Object.keys(stop.arrivals).flatMap(type =>
      Object.keys(stop.arrivals[type]).map(route => `${type}-${route}`)
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
    <button class="remove-btn" onclick={() => dispatch('remove')}>✕</button>
    <button class="filter-btn" onclick={() => showFilters = !showFilters}>⚙️</button>
  </div>

  {#if showFilters}
    <div class="filters">
      {#each Object.keys(stop.arrivals) as type}
        {#each Object.keys(stop.arrivals[type]) as route}
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
    {#if Object.keys(stop.arrivals).length === 0}
      <p>No arrivals available at this time.</p>
    {:else}
      {#each Object.keys(stop.arrivals) as type}
        {#each Object.keys(stop.arrivals[type]) as route}
          {#if !hiddenRoutes.includes(route)}
            <RouteArrivals {type} {route} arrivals={stop.arrivals[type][route]} />
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