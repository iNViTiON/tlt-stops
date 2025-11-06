<script lang="ts">
  import { onMount } from 'svelte';
  import { currentTime } from './lib/stores';
  import type { StopArrival, FavoriteStop, RawStopArrival } from './lib/types';
  import StopCard from './StopCard.svelte';

  let stops = $state<StopArrival[]>([]);
  let loading = $state(false);
  let hiddenRoutes = $state(loadHiddenRoutes());
  let nextUpdateTimes = $state<{[stopId: string]: number}>({});

  const API_BASE = '/api';

  function calculateNextUpdateDelay(firstArrivalMinutes: number): number {
    if (firstArrivalMinutes > 10) return 60_000;
    if (firstArrivalMinutes >= 5) return 30_000;
    if (firstArrivalMinutes >= 3) return 15_000;
    if (firstArrivalMinutes >= 2) return 10_000;
    return 5_000;
  }

  function getFirstArrivalTime(stop: StopArrival, stopId: string): number {
    const hidden = hiddenRoutes[stopId] || [];
    let minTime: number = Number.MAX_SAFE_INTEGER;
    for (const types of Object.values(stop.arrivals)) {
      for (const [route, arrivals] of Object.entries(types)) {
        if (hidden.includes(route)) continue;
        for (const arrival of arrivals) {
          minTime = Math.min(minTime, arrival.time);
        }
      }
    }
    return minTime;
  }

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
    const now = Date.now();
    for (const chunk of chunks) {
      const ids = chunk.map(f => f.id);
      const response = await fetch(`${API_BASE}/arrivals?stops=${ids.join(',')}`);
      if (response.ok) {
        const data: { stops: (RawStopArrival | null)[] } = await response.json();
        for (let i = 0; i < data.stops.length; i++) {
          const rawStop = data.stops[i];
          if (rawStop && rawStop.arrivals) {
            // Convert ISO time strings to timestamps
            const arrivals: StopArrival['arrivals'] = {};
            for (const [type, routes] of Object.entries(rawStop.arrivals)) {
              arrivals[type] = {};
              for (const [route, arrivalList] of Object.entries(routes)) {
                arrivals[type][route] = arrivalList.map(a => ({
                  time: new Date(a.time).getTime(),
                  timeString: a.time,
                  isLowEntry: a.isLowEntry
                }));
              }
            }
            
            const stopData: StopArrival = {
              id: chunk[i].id,
              name: chunk[i].name,
              arrivals
            };
            results.push(stopData);
            
            // Calculate next update time based on first visible arrival
            const firstArrival = getFirstArrivalTime(stopData, chunk[i].id);
            if (firstArrival !== null) {
              const minutesUntilArrival = Math.floor((firstArrival - now) / 60000);
              const delay = calculateNextUpdateDelay(minutesUntilArrival);
              nextUpdateTimes[chunk[i].id] = now + delay;
            } else {
              // No arrivals, check again in 60 seconds
              nextUpdateTimes[chunk[i].id] = now + 60000;
            }
          }
        }
      }
    }
    return results;
  }

  async function refreshData(isInitial = false) {
    if (isInitial) {
      loading = true;
      const favorites = loadFavorites();
      stops = await fetchArrivals(favorites);
      loading = false;
    } else {
      // Smart update: only fetch stops that need updating
      const favorites = loadFavorites();
      const now = Date.now();
      const stopsToUpdate = favorites.filter(f => !nextUpdateTimes[f.id] || nextUpdateTimes[f.id] <= now);
      
      if (stopsToUpdate.length > 0) {
        const updatedStops = await fetchArrivals(stopsToUpdate);
        
        // Merge updated stops with existing ones
        const stopMap = new Map(stops.map(s => [s.id, s]));
        updatedStops.forEach(s => stopMap.set(s.id, s));
        stops = Array.from(stopMap.values());
      }
    }
  }

  onMount(() => {
    refreshData(true);
    const interval = setInterval(() => refreshData(false), 5000); // Check every 5 seconds
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