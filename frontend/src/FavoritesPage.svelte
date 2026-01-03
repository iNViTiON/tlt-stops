<script lang="ts">
  import { onMount } from "svelte";
  import { currentTime } from "./lib/stores";
  import type { StopArrival, FavoriteStop, RawStopArrival } from "./lib/types";
  import StopCard from "./StopCard.svelte";

  let stops = $state<StopArrival[]>([]);
  let loading = $state(false);
  let hiddenRoutes = $state(loadHiddenRoutes());
  let nextUpdateTimes = $state<{ [stopId: string]: number }>({});

  const API_BASE = "/api";

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
    const stored = localStorage.getItem("favoriteStops");
    return stored ? JSON.parse(stored) : [];
  }

  function loadHiddenRoutes(): { [stopId: string]: string[] } {
    const stored = localStorage.getItem("hiddenRoutes");
    return stored ? JSON.parse(stored) : {};
  }

  async function fetchArrivals(
    favorites: FavoriteStop[]
  ): Promise<StopArrival[]> {
    if (favorites.length === 0) return [];

    const chunks: FavoriteStop[][] = [];
    for (let i = 0; i < favorites.length; i += 5) {
      chunks.push(favorites.slice(i, i + 5));
    }

    const results: StopArrival[] = [];
    const now = Date.now();
    for (const chunk of chunks) {
      const ids = chunk.map((f) => f.id);
      try {
        const response = await fetch(
          `${API_BASE}/arrivals?stops=${ids.join(",")}`
        );
        if (response.ok) {
          const data: { stops: (RawStopArrival | null)[] } =
            await response.json();
          for (let i = 0; i < data.stops.length; i++) {
            const rawStop = data.stops[i];
            if (rawStop && rawStop.arrivals) {
              const arrivals: StopArrival["arrivals"] = {};
              for (const [type, routes] of Object.entries(rawStop.arrivals)) {
                arrivals[type] = {};
                for (const [route, arrivalList] of Object.entries(routes)) {
                  arrivals[type][route] = arrivalList.map((a) => ({
                    time: new Date(a.time).getTime(),
                    timeString: a.time,
                    isLowEntry: a.isLowEntry,
                  }));
                }
              }

              const stopData: StopArrival = {
                id: chunk[i].id,
                name: chunk[i].name,
                arrivals,
              };
              results.push(stopData);

              const firstArrival = getFirstArrivalTime(stopData, chunk[i].id);
              if (firstArrival !== Number.MAX_SAFE_INTEGER) {
                const minutesUntilArrival = Math.floor(
                  (firstArrival - now) / 60000
                );
                const delay = calculateNextUpdateDelay(minutesUntilArrival);
                nextUpdateTimes[chunk[i].id] = now + delay;
              } else {
                nextUpdateTimes[chunk[i].id] = now + 60000;
              }
            }
          }
        }
      } catch (e) {
        console.error("Fetch error:", e);
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
      const favorites = loadFavorites();
      const now = Date.now();
      const stopsToUpdate = favorites.filter(
        (f) => !nextUpdateTimes[f.id] || nextUpdateTimes[f.id] <= now
      );

      if (stopsToUpdate.length > 0) {
        const updatedStops = await fetchArrivals(stopsToUpdate);
        const stopMap = new Map(stops.map((s) => [s.id, s]));
        updatedStops.forEach((s) => stopMap.set(s.id, s));
        stops = Array.from(stopMap.values());
      }
    }
  }

  onMount(() => {
    refreshData(true);
    const interval = setInterval(() => refreshData(false), 5000);
    return () => clearInterval(interval);
  });

  function removeFavorite(stopId: string) {
    const favorites = loadFavorites().filter((f) => f.id !== stopId);
    localStorage.setItem("favoriteStops", JSON.stringify(favorites));
    const hidden = loadHiddenRoutes();
    if (hidden[stopId]) {
      delete hidden[stopId];
      localStorage.setItem("hiddenRoutes", JSON.stringify(hidden));
      hiddenRoutes = hidden;
    }
    stops = stops.filter((s) => s.id !== stopId);
    if (nextUpdateTimes[stopId]) {
      delete nextUpdateTimes[stopId];
    }
  }

  function toggleHiddenRoute(stopId: string, route: string) {
    if (!hiddenRoutes[stopId]) hiddenRoutes[stopId] = [];
    const index = hiddenRoutes[stopId].indexOf(route);
    if (index > -1) {
      hiddenRoutes[stopId].splice(index, 1);
    } else {
      hiddenRoutes[stopId].push(route);
    }
    localStorage.setItem("hiddenRoutes", JSON.stringify(hiddenRoutes));
  }
</script>

<div class="favorites">
  {#if loading}
    <div class="loading-state">
      <div class="skeleton-card"></div>
      <div class="skeleton-card"></div>
    </div>
  {:else if stops.length === 0}
    <div class="empty-state">
      <div class="empty-icon">⭐</div>
      <h2>No favorite stops yet</h2>
      <p>
        Add your most used stops from the Browse tab to see their arrivals here
        instantly.
      </p>
    </div>
  {:else}
    <div class="stops-list">
      {#each stops as stop (stop.id)}
        <StopCard
          {stop}
          hiddenRoutes={hiddenRoutes[stop.id] || []}
          on:remove={() => removeFavorite(stop.id)}
          on:toggleHidden={(e) => toggleHiddenRoute(stop.id, e.detail)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .favorites {
    padding-bottom: 2rem;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .skeleton-card {
    height: 200px;
    background: var(--card-bg);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    position: relative;
    overflow: hidden;
  }

  .skeleton-card::after {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: linear-gradient(
      90deg,
      transparent,
      rgba(255, 255, 255, 0.05),
      transparent
    );
    animation: shimmer 2s infinite;
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  .empty-state {
    text-align: center;
    padding: 4rem 2rem;
    background: var(--card-bg);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .empty-state h2 {
    font-size: 1.25rem;
    font-weight: 700;
  }

  .empty-state p {
    color: var(--secondary-text);
    max-width: 320px;
    line-height: 1.6;
  }

  .stops-list {
    display: flex;
    flex-direction: column;
  }
</style>
