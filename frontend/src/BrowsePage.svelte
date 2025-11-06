<script lang="ts">
  import { onMount } from 'svelte';
  import StopCard from './StopCard.svelte';
  import type { StopArrival, FavoriteStop } from './lib/types';

  let types = $state<string[]>([]);
  let routes = $state<string[]>([]);
  let directions = $state<string[]>([]);
  let stops = $state<Array<[string, string]>>([]); // [id, name]

  let selectedType = $state('');
  let selectedRoute = $state('');
  let selectedDirection = $state('');
  let selectedStopId = $state('');

  let stopData = $state<StopArrival | null>(null);
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

  function isFavorite(stopId: string): boolean {
    return loadFavorites().some(f => f.id === stopId);
  }

  function toggleFavorite(stop: StopArrival) {
    const favorites = loadFavorites();
    const existing = favorites.find(f => f.id === stop.id);
    if (existing) {
      // remove
      const newFavs = favorites.filter(f => f.id !== stop.id);
      localStorage.setItem('favoriteStops', JSON.stringify(newFavs));
    } else {
      // add, but limit to 5 favorites
      if (favorites.length >= 5) {
        alert('You can only have up to 5 favorite stops.');
        return;
      }
      favorites.push({ id: stop.id, name: stop.name });
      localStorage.setItem('favoriteStops', JSON.stringify(favorites));
      // initialize hidden routes
      const hidden = JSON.parse(localStorage.getItem('hiddenRoutes') || '{}');
      if (!hidden[stop.id]) {
        hidden[stop.id] = [];
        localStorage.setItem('hiddenRoutes', JSON.stringify(hidden));
      }
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
    localStorage.setItem('hiddenRoutes', JSON.stringify(hiddenRoutes));
  }

  async function fetchTypes() {
    const response = await fetch(`${API_BASE}/types`);
    if (response.ok) {
      types = await response.json();
      // mark as loaded
    }
  }

  async function fetchRoutesFor(type: string) {
    const response = await fetch(`${API_BASE}/types/${type}/routes`);
    if (response.ok) {
      routes = await response.json();
    }
  }

  async function fetchDirectionsFor(type: string, route: string) {
    const response = await fetch(`${API_BASE}/types/${type}/routes/${route}/directions`);
    if (response.ok) {
      directions = await response.json();
    }
  }

  async function fetchStopsFor(type: string, route: string, direction: string) {
    const response = await fetch(`${API_BASE}/types/${type}/routes/${route}/directions/${encodeURIComponent(direction)}/stops`);
    if (response.ok) {
      stops = await response.json();
    }
  }

  async function fetchArrivalsFor(stopId: string) {
    const response = await fetch(`${API_BASE}/arrivals?stops=${stopId}`);
    if (response.ok) {
      const data: { stops: (StopArrival | null)[] } = await response.json();
      stopData = data.stops[0] || null;
    }
  }

  async function fetchStopData() {
    if (!selectedStopId) return;
    console.log('selectedStopId:', selectedStopId);
    loading = true;
    try {
      const response = await fetch(`${API_BASE}/arrivals?stops=${selectedStopId}`);
      if (response.ok) {
        const data: { stops: (StopArrival | null)[] } = await response.json();
        console.log('API response data:', data);
        stopData = data.stops[0] || null;
        console.log('Selected stopData:', stopData);
      } else {
        console.error('Failed to fetch arrivals:', response.status, response.statusText);
        stopData = null;
      }
    } catch (error) {
      console.error('Error fetching arrivals:', error);
      stopData = null;
    }
    loading = false;
  }  function updateURL() {
    const params = new URLSearchParams();
    if (selectedType) params.set('type', selectedType);
    if (selectedRoute) params.set('route', selectedRoute);
    if (selectedDirection) params.set('direction', selectedDirection);
    if (selectedStopId) params.set('stop', selectedStopId);
    const newURL = `/browse?${params.toString()}`;
    history.replaceState({}, '', newURL);
  }

  function parseURL() {
    const params = new URLSearchParams(window.location.search);
    selectedType = params.get('type') || '';
    selectedRoute = params.get('route') || '';
    selectedDirection = params.get('direction') || '';
    selectedStopId = params.get('stop') || '';
  }

  // Remove reactive blocks for clearing, handle in onchange

  onMount(async () => {
    // Initial load sequence: fetch options and validate any values from the URL
    // parse raw URL params first
    const params = new URLSearchParams(window.location.search);
    const urlType = params.get('type') || '';
    const urlRoute = params.get('route') || '';
    const urlDirection = params.get('direction') || '';
    const urlStop = params.get('stop') || '';

    // Set selectedStopId early if stop in URL, to show arrivals immediately when loaded
    if (urlStop) {
      selectedStopId = urlStop;
    }

    // Start arrivals fetch concurrently if stop is in URL
    const arrivalsPromise = urlStop ? fetchArrivalsFor(urlStop) : Promise.resolve();

    // Sequential choices restoration
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
            const stopIds = stops.map(s => s[0]);
            if (stopIds.includes(urlStop)) {
              // selectedStopId already set, ensure arrivals are loaded
              await arrivalsPromise;
            } else {
              // invalid stop in URL; clear it
              selectedStopId = '';
            }
          }
        } else {
          // invalid direction in URL; clear downstream
          selectedDirection = '';
          selectedStopId = '';
        }
      } else {
        // invalid route in URL; clear downstream
        selectedRoute = '';
        selectedDirection = '';
        selectedStopId = '';
      }
    } else {
      // invalid or missing type: clear all URL-driven selections
      selectedType = '';
      selectedRoute = '';
      selectedDirection = '';
      selectedStopId = '';
    }

    // If arrivals were fetched but stop is invalid, clear stopData
    if (urlStop && !selectedStopId) {
      stopData = null;
    }

    // After validation, ensure the URL reflects only valid selections
    updateURL();
  });
</script>

<div class="browse">
  <div class="selectors">
    <label>
      Transport Type:
      <select bind:value={selectedType} onchange={async () => { routes = []; selectedRoute = ''; selectedDirection = ''; selectedStopId = ''; stopData = null; await fetchRoutesFor(selectedType); updateURL(); }}>
        <option value="">Select type</option>
        {#each types as type}
          <option value={type}>{type}</option>
        {/each}
      </select>
    </label>

    <label>
      Route:
      <select bind:value={selectedRoute} onchange={async () => { directions = []; selectedDirection = ''; selectedStopId = ''; stopData = null; await fetchDirectionsFor(selectedType, selectedRoute); updateURL(); }} disabled={!selectedType}>
        <option value="">Select route</option>
        {#each routes as route}
          <option value={route}>{route}</option>
        {/each}
      </select>
    </label>

    <label>
      Direction:
            <select bind:value={selectedDirection} onchange={async () => { stops = []; selectedStopId = ''; stopData = null; await fetchStopsFor(selectedType, selectedRoute, selectedDirection); updateURL(); }} disabled={!selectedRoute}>
        <option value="">Select direction</option>
        {#each directions as direction}
          <option value={direction}>{direction}</option>
        {/each}
      </select>
    </label>

    <label>
      Stop:
      <select bind:value={selectedStopId} onchange={async () => { stopData = null; await fetchArrivalsFor(selectedStopId); updateURL(); }} disabled={!selectedDirection}>
        <option value="">Select stop</option>
        {#each stops as [id, name]}
          <option value={id}>{name}</option>
        {/each}
      </select>
    </label>
  </div>

  {#if selectedStopId}
    {#if loading}
      <p>Loading arrivals...</p>
    {:else if stopData}
      <div class="result">
        <button class="favorite-btn" onclick={() => toggleFavorite(stopData)}>
          {isFavorite(selectedStopId) ? '★' : '☆'} {isFavorite(selectedStopId) ? 'Favorited' : 'Add to Favorites'}
        </button>
        <StopCard 
          stop={stopData} 
          hiddenRoutes={hiddenRoutes[selectedStopId] || []}
          on:toggleHidden={(e) => toggleHiddenRoute(selectedStopId, e.detail)}
        />
      </div>
    {/if}
  {/if}
</div>

<style>
  .browse {
    padding: 1rem;
    padding-bottom: 5rem;
  }

  .selectors {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  select {
    width: 100%;
    padding: 0.5rem;
    font-size: 1rem;
  }

  .result {
    margin-top: 2rem;
  }

  .favorite-btn {
    padding: 0.5rem 1rem;
    font-size: 1rem;
    margin-bottom: 1rem;
  }
</style>