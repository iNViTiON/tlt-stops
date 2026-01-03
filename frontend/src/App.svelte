<script lang="ts">
  import { onMount } from "svelte";
  import Nav from "./Nav.svelte";
  import FavoritesPage from "./FavoritesPage.svelte";
  import BrowsePage from "./BrowsePage.svelte";

  let currentRoute = $state(window.location.pathname);
  let theme = $state("light");

  function navigate(path: string) {
    history.pushState({}, "", path);
    currentRoute = path;
  }

  function toggleTheme() {
    theme = theme === "light" ? "dark" : "light";
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("theme", theme);
  }

  window.addEventListener("popstate", () => {
    currentRoute = window.location.pathname;
  });

  onMount(() => {
    const savedTheme = localStorage.getItem("theme");
    const prefersDark = window.matchMedia(
      "(prefers-color-scheme: dark)"
    ).matches;

    theme = savedTheme || (prefersDark ? "dark" : "light");
    document.documentElement.setAttribute("data-theme", theme);
  });
</script>

<div class="container">
  <header class="header">
    <h1>TLT Stops</h1>
    <button
      class="secondary theme-toggle"
      onclick={toggleTheme}
      aria-label="Toggle theme"
    >
      {theme === "light" ? "🌙" : "☀️"}
    </button>
  </header>

  <main id="app">
    {#if currentRoute === "/"}
      <FavoritesPage />
    {:else if currentRoute.startsWith("/browse")}
      <BrowsePage />
    {/if}
  </main>
</div>

<Nav {currentRoute} {navigate} />

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 2rem 0;
  }

  h1 {
    font-size: 1.5rem;
    font-weight: 800;
    letter-spacing: -0.025em;
  }

  .theme-toggle {
    padding: 0.5rem;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 50%;
  }

  #app {
    padding-bottom: 5rem; /* space for nav */
  }
</style>
