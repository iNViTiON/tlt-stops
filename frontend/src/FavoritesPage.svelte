<script lang="ts">
  import { onMount } from 'svelte';
  import { currentTime } from './lib/stores';
  import type { StopArrival, FavoriteStop } from './lib/types';
  import StopCard from './StopCard.svelte';

  let stops = $state<StopArrival[]>([]);
  let loading = $state(false);
  let hiddenRoutes = $state(loadHiddenRoutes());

  const API_BASE = '/api';

  function loadFavorites(): FavoriteStop[] {
    const stored = localStorage.getItem('favoriteStops');
    return stored ? JSON.parse(stored) : [];
  }

  function loadHiddenRoutes(): { [stopId: string]: string[] } {
    const stored = localStorage.getItem('hiddenRoutes');
    return stored ? JSON.parse(stored) : {};
  }

  async function fetchArrivals(favorites: FavoriteStop[]): Promise<StopArrival[]> {
    if (favorites.length === 0) return [];

    const chunks: FavoriteStop[][] = [];
    for (let i = 0; i < favorites.length; i += 5) {
      chunks.push(favorites.slice(i, i + 5));
    }

    const results: StopArrival[] = [];
    for (const chunk of chunks) {
      const ids = chunk.map(f => f.id);
      const response = await fetch(`${API_BASE}/arrivals?stops=${ids.join(',')}`);
      if (response.ok) {
        const data: { stops: (StopArrival | null)[] } = await response.json();
        for (let i = 0; i < data.stops.length; i++) {
          if (data.stops[i]) {
            results.push({
              ...data.stops[i],
              id: chunk[i].id,
              name: chunk[i].name
            });
          }
        }
      }
    }
    return results;
  }

  async function refreshData(isInitial = false) {
    if (isInitial) loading = true;
    const favorites = loadFavorites();
    stops = await fetchArrivals(favorites);
    if (isInitial) loading = false;
  }

  onMount(() => {
    refreshData(true);
    const interval = setInterval(() => refreshData(false), 30000); // 30 seconds
    return () => clearInterval(interval);
  });

  function removeFavorite(stopId: string) {
    const favorites = loadFavorites().filter(f => f.id !== stopId);
    localStorage.setItem('favoriteStops', JSON.stringify(favorites));
    stops = stops.filter(s => s.id !== stopId);
  }

  function toggleHiddenRoute(stopId: string, route: string) {
    if (!hiddenRoutes[stopId]) hiddenRoutes[stopId] = [];
    const index = hiddenRoutes[stopId].indexOf(route);
    if (index > -1) {
      hiddenRoutes[stopId].splice(index, 1);
    } else {
      hiddenRoutes[stopId].push(route);
    }
    localStorage.setItem('hiddenRoutes', JSON.stringify(hiddenRoutes));
    // No need for force update, reactive state will handle it
  }
</script>

<div class="favorites">
  {#if loading}
    <p>Loading...</p>
  {:else if stops.length === 0}
    <p>No favorite stops yet. Add some from the Browse tab!</p>
  {:else}
    {#each stops as stop}
      <StopCard
        {stop}
        hiddenRoutes={hiddenRoutes[stop.id] || []}
        on:remove={() => removeFavorite(stop.id)}
        on:toggleHidden={(e) => toggleHiddenRoute(stop.id, e.detail)}
      />
    {/each}
  {/if}
</div>

<style>
  .favorites {
    padding: 1rem;
    padding-bottom: 5rem; /* space for nav */
  }
</style>