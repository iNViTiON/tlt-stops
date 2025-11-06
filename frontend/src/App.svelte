<script lang="ts">
  import { onMount } from 'svelte';
  import Nav from './Nav.svelte';
  import FavoritesPage from './FavoritesPage.svelte';
  import BrowsePage from './BrowsePage.svelte';

  let currentRoute = $state(window.location.pathname);

  function navigate(path: string) {
    history.pushState({}, '', path);
    currentRoute = path;
  }

  window.addEventListener('popstate', () => {
    currentRoute = window.location.pathname;
  });

  onMount(() => {
    // Set theme based on prefers-color-scheme
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      document.documentElement.setAttribute('data-theme', 'dark');
    }
  });
</script>

<div id="app">
  {#if currentRoute === '/'}
    <FavoritesPage />
  {:else if currentRoute.startsWith('/browse')}
    <BrowsePage />
  {/if}
</div>

<Nav {currentRoute} {navigate} />

<style>
  #app {
    padding-bottom: 5rem; /* space for nav */
  }
</style>
