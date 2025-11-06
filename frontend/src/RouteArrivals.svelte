<script lang="ts">
  import { currentTime } from './lib/stores';
  export let type: string;
  export let route: string;
  export let arrivals: Array<{ time: number; timeString?: string; isLowEntry?: boolean }>;

  function getIcon(type: string): string {
    switch (type) {
      case 'bus': return '🚌';
      case 'commercialbus': return '🚍';
      case 'regionalbus': return '🚍';
      case 'train': return '🚆';
      case 'tram': return '🚋';
      case 'trolleybus': return '🚎';
      default: return '❓';
    }
  }

  $: countdowns = arrivals.map(a => {
    const now = $currentTime;
    const diff = Math.floor((a.time - now) / 1000);
    if (diff <= 0) return 'Now';
    const minutes = Math.floor(diff / 60);
    const seconds = diff % 60;
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  });
</script>

<div class="route-arrivals">
  <div class="route-header">
    <span class="icon">{getIcon(type)}</span>
    <span class="route-number">{route}</span>
  </div>
  <div class="times">
    {#each arrivals as arrival, i}
      <span class="time">
        {countdowns[i]}{#if arrival.isLowEntry}♿{/if}
      </span>
    {/each}
  </div>
</div>

<style>
  .route-arrivals {
    display: flex;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .route-header {
    display: flex;
    align-items: center;
    min-width: 4rem;
  }

  .icon {
    font-size: 1.5rem;
    margin-right: 0.5rem;
  }

  .route-number {
    font-weight: bold;
  }

  .times {
    display: flex;
    margin-left: 1rem;
    overflow-x: auto;
    white-space: nowrap;
    scrollbar-width: thin;
    scrollbar-color: var(--border-color) transparent;
  }

  .time {
    margin-right: 1rem;
    font-family: monospace;
    flex-shrink: 0;
  }
</style>