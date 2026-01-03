<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import StopCard from "./StopCard.svelte";
  import type { StopArrival, FavoriteStop, RawStopArrival } from "./lib/types";

  let types = $state<string[]>([]);
  let routes = $state<string[]>([]);
  let directions = $state<string[]>([]);
  let stops = $state<Array<[string, string]>>([]);

  let selectedType = $state("");
  let selectedRoute = $state("");
  let selectedDirection = $state("");
  let selectedStopId = $state("");

  let stopData = $state<StopArrival | null>(null);
  let loading = $state(false);
  let hiddenRoutes = $state(loadHiddenRoutes());
  let favorites = $state<FavoriteStop[]>(loadFavorites());
  let nextUpdateTime = $state<number | null>(null);

  const API_BASE = "/api";

  function calculateNextUpdateDelay(firstArrivalMinutes: number): number {
    if (firstArrivalMinutes > 10) return 60_000;
    if (firstArrivalMinutes >= 5) return 30_000;
    if (firstArrivalMinutes >= 3) return 15_000;
    if (firstArrivalMinutes >= 2) return 10_000;
    return 5_000;
  }

  function getFirstArrivalTime(
    stop: StopArrival,
    stopId: string
  ): number | null {
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
    return minTime === Number.MAX_SAFE_INTEGER ? null : minTime;
  }

  function loadFavorites(): FavoriteStop[] {
    const stored = localStorage.getItem("favoriteStops");
    return stored ? JSON.parse(stored) : [];
  }

  function loadHiddenRoutes(): { [stopId: string]: string[] } {
    const stored = localStorage.getItem("hiddenRoutes");
    return stored ? JSON.parse(stored) : {};
  }

  function isFavorite(stopId: string): boolean {
    if (!stopId) return false;
    const id = String(stopId).trim();
    return favorites.some((f) => String(f.id).trim() === id);
  }

  function toggleFavorite(stop: StopArrival) {
    const id = String(stop.id || selectedStopId || "").trim();
    if (!id) return;
    const existing = favorites.find((f) => String(f.id).trim() === id);
    if (existing) {
      favorites = favorites.filter((f) => String(f.id).trim() !== id);
      localStorage.setItem("favoriteStops", JSON.stringify(favorites));
      const hidden = JSON.parse(localStorage.getItem("hiddenRoutes") || "{}");
      if (hidden[id]) {
        delete hidden[id];
        localStorage.setItem("hiddenRoutes", JSON.stringify(hidden));
        hiddenRoutes = hidden;
      }
    } else {
      if (favorites.length >= 5) {
        alert("You can only have up to 5 favorite stops.");
        return;
      }
      favorites = [...favorites, { id, name: stop.name }];
      localStorage.setItem("favoriteStops", JSON.stringify(favorites));
      const hidden = JSON.parse(localStorage.getItem("hiddenRoutes") || "{}");
      if (!hidden[id]) {
        hidden[id] = [];
        localStorage.setItem("hiddenRoutes", JSON.stringify(hidden));
        hiddenRoutes = hidden;
      }
      window.dispatchEvent(new PopStateEvent("popstate"));
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

  async function fetchTypes() {
    const response = await fetch(`${API_BASE}/types`);
    if (response.ok) {
      types = await response.json();
    }
  }

  async function fetchRoutesFor(type: string) {
    if (!type) return;
    const response = await fetch(`${API_BASE}/types/${type}/routes`);
    if (response.ok) {
      routes = await response.json();
    }
  }

  async function fetchDirectionsFor(type: string, route: string) {
    if (!type || !route) return;
    const response = await fetch(
      `${API_BASE}/types/${type}/routes/${route}/directions`
    );
    if (response.ok) {
      directions = await response.json();
    }
  }

  async function fetchStopsFor(type: string, route: string, direction: string) {
    if (!type || !route || !direction) return;
    const response = await fetch(
      `${API_BASE}/types/${type}/routes/${route}/directions/${encodeURIComponent(direction)}/stops`
    );
    if (response.ok) {
      stops = await response.json();
    }
  }

  async function fetchStopData(showLoading: boolean = true) {
    if (!selectedStopId) return;
    if (showLoading) loading = true;
    try {
      const response = await fetch(
        `${API_BASE}/arrivals?stops=${selectedStopId}`
      );
      if (response.ok) {
        const data: { stops: (RawStopArrival | null)[] } =
          await response.json();
        const rawStop = data.stops[0];
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
          stopData = {
            id: selectedStopId,
            name: rawStop.name || "",
            arrivals,
          };
          const firstArrival = getFirstArrivalTime(stopData, selectedStopId);
          if (firstArrival !== null) {
            const minutesUntilArrival = Math.floor(
              (firstArrival - Date.now()) / 60000
            );
            const delay = calculateNextUpdateDelay(minutesUntilArrival);
            nextUpdateTime = Date.now() + delay;
          } else {
            nextUpdateTime = Date.now() + 60000;
          }
        } else {
          stopData = null;
        }
      } else {
        stopData = null;
      }
    } catch (error) {
      stopData = null;
    }
    if (showLoading) loading = false;
  }

  function updateURL() {
    const params = new URLSearchParams();
    if (selectedType) params.set("type", selectedType);
    if (selectedRoute) params.set("route", selectedRoute);
    if (selectedDirection) params.set("direction", selectedDirection);
    if (selectedStopId) params.set("stop", selectedStopId);
    const newURL = `/browse?${params.toString()}`;
    history.replaceState({}, "", newURL);
  }

  onMount(async () => {
    const params = new URLSearchParams(window.location.search);
    const urlType = params.get("type") || "";
    const urlRoute = params.get("route") || "";
    const urlDirection = params.get("direction") || "";
    const urlStop = params.get("stop") || "";

    if (urlStop) {
      selectedStopId = urlStop;
    }

    const arrivalsPromise = urlStop ? fetchStopData(false) : Promise.resolve();

    await fetchTypes();
    if (urlType && types.includes(urlType)) {
      selectedType = urlType;
      await fetchRoutesFor(urlType);
      if (urlRoute && routes.includes(urlRoute)) {
        selectedRoute = urlRoute;
        await fetchDirectionsFor(urlType, urlRoute);
        if (urlDirection && directions.includes(urlDirection)) {
          selectedDirection = urlDirection;
          await fetchStopsFor(urlType, urlRoute, urlDirection);
          if (urlStop) {
            const stopIds = stops.map((s) => s[0]);
            if (stopIds.includes(urlStop)) {
              await arrivalsPromise;
            } else {
              selectedStopId = "";
            }
          }
        } else {
          selectedDirection = "";
          selectedStopId = "";
        }
      } else {
        selectedRoute = "";
        selectedDirection = "";
        selectedStopId = "";
      }
    } else {
      selectedType = "";
      selectedRoute = "";
      selectedDirection = "";
      selectedStopId = "";
    }

    if (urlStop && !selectedStopId) {
      stopData = null;
    }

    updateURL();
  });

  let interval: number | undefined;

  onMount(() => {
    interval = setInterval(() => {
      if (selectedStopId && nextUpdateTime && Date.now() >= nextUpdateTime) {
        fetchStopData(false);
      }
    }, 5000);
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
  });
</script>

<div class="browse">
  <div class="selectors card">
    <div class="field">
      <label for="type">Transport Type</label>
      <select
        id="type"
        bind:value={selectedType}
        onchange={async () => {
          routes = [];
          selectedRoute = "";
          selectedDirection = "";
          selectedStopId = "";
          stopData = null;
          nextUpdateTime = null;
          await fetchRoutesFor(selectedType);
          updateURL();
        }}
      >
        <option value="">Select type</option>
        {#each types as type}
          <option value={type}>{type}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <label for="route">Route</label>
      <select
        id="route"
        bind:value={selectedRoute}
        onchange={async () => {
          directions = [];
          selectedDirection = "";
          selectedStopId = "";
          stopData = null;
          nextUpdateTime = null;
          await fetchDirectionsFor(selectedType, selectedRoute);
          updateURL();
        }}
        disabled={!selectedType}
      >
        <option value="">Select route</option>
        {#each routes as route}
          <option value={route}>{route}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <label for="direction">Direction</label>
      <select
        id="direction"
        bind:value={selectedDirection}
        onchange={async () => {
          stops = [];
          selectedStopId = "";
          stopData = null;
          nextUpdateTime = null;
          await fetchStopsFor(selectedType, selectedRoute, selectedDirection);
          updateURL();
        }}
        disabled={!selectedRoute}
      >
        <option value="">Select direction</option>
        {#each directions as direction}
          <option value={direction}>{direction}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <label for="stop">Stop</label>
      <select
        id="stop"
        bind:value={selectedStopId}
        onchange={async () => {
          stopData = null;
          nextUpdateTime = null;
          await fetchStopData(true);
          updateURL();
        }}
        disabled={!selectedDirection}
      >
        <option value="">Select stop</option>
        {#each stops as [id, name]}
          <option value={id}>{name}</option>
        {/each}
      </select>
    </div>
  </div>

  {#if selectedStopId}
    <div class="result">
      {#if loading}
        <div class="loading-container">
          <p>Fetching arrivals...</p>
        </div>
      {:else if stopData}
        <div class="actions">
          <button
            class="favorite-btn"
            class:is-favorite={isFavorite(selectedStopId)}
            onclick={() => stopData && toggleFavorite(stopData)}
          >
            <span class="star">{isFavorite(selectedStopId) ? "★" : "☆"}</span>
            {isFavorite(selectedStopId) ? "Favorited" : "Add to Favorites"}
          </button>
        </div>
        <StopCard
          stop={stopData}
          hiddenRoutes={hiddenRoutes[selectedStopId] || []}
          {selectedRoute}
          browsing={true}
          on:toggleHidden={(e) => toggleHiddenRoute(selectedStopId, e.detail)}
        />
      {/if}
    </div>
  {/if}
</div>

<style>
  .browse {
    padding-bottom: 2rem;
  }

  .card {
    background: var(--card-bg);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1.5rem;
    box-shadow: var(--shadow-sm);
  }

  .selectors {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .field label {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--secondary-text);
  }

  select {
    width: 100%;
    padding: 0.75rem;
    font-size: 1rem;
    background: var(--bg-color);
    color: var(--text-color);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 24 24' stroke='currentColor'%3E%3Cpath stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='C19 9l-7 7-7-7'%3E%3C/path%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.75rem center;
    background-size: 1rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  select:focus {
    outline: none;
    border-color: var(--accent-color);
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.1);
  }

  select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    background-color: var(--bg-color);
  }

  .result {
    margin-top: 2rem;
  }

  .loading-container {
    text-align: center;
    padding: 3rem;
    color: var(--secondary-text);
  }

  .actions {
    margin-bottom: 1rem;
  }

  .favorite-btn {
    width: 100%;
    padding: 1rem;
    font-size: 1rem;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    background: var(--card-bg);
    color: var(--text-color);
    border: 1px solid var(--border-color);
  }

  .favorite-btn.is-favorite {
    background: rgba(59, 130, 246, 0.1);
    border-color: var(--accent-color);
    color: var(--accent-color);
  }

  .favorite-btn .star {
    font-size: 1.25rem;
    line-height: 1;
  }
</style>
