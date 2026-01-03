<script lang="ts">
  export let currentRoute: string;
  export let navigate: (path: string) => void;
</script>

<nav class="nav">
  <div class="nav-container">
    <button
      class="nav-button {currentRoute === '/' ? 'active' : ''}"
      onclick={() => navigate("/")}
    >
      <span class="icon">⭐</span>
      <span class="label">Favorites</span>
    </button>
    <button
      class="nav-button {currentRoute.startsWith('/browse') ? 'active' : ''}"
      onclick={() => navigate("/browse")}
    >
      <span class="icon">🔍</span>
      <span class="label">Browse</span>
    </button>
  </div>
</nav>

<style>
  .nav {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    background: var(--card-bg);
    border-top: 1px solid var(--border-color);
    padding: 0.75rem 0 calc(0.75rem + env(safe-area-inset-bottom));
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    z-index: 100;
  }

  .nav-container {
    max-width: 640px;
    margin: 0 auto;
    display: flex;
    justify-content: space-around;
    padding: 0 1rem;
  }

  .nav-button {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    background: none;
    border: none;
    padding: 0.5rem;
    color: var(--secondary-text);
    transition: all 0.2s ease;
    cursor: pointer;
    box-shadow: none;
    transform: none;
  }

  .nav-button:hover {
    background: none;
    box-shadow: none;
    transform: none;
    color: var(--text-color);
  }

  .nav-button.active {
    color: var(--accent-color);
  }

  .icon {
    font-size: 1.25rem;
  }

  .label {
    font-size: 0.75rem;
    font-weight: 500;
  }
</style>
