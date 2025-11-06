<script lang="ts">
  export let currentRoute: string;
  export let navigate: (path: string) => void;
</script>

<nav class="nav">
  <button class="nav-button {currentRoute === '/' ? 'active' : ''}" onclick={() => navigate('/')}>
    ⭐ Favorites
  </button>
  <button class="nav-button {currentRoute === '/browse' ? 'active' : ''}" onclick={() => navigate('/browse')}>
    🔍 Browse
  </button>
</nav>

<style>
  .nav {
    display: flex;
    justify-content: space-around;
    padding: 1rem;
    background: var(--bg-color);
    border-top: 1px solid var(--border-color);
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
  }

  .nav-button {
    flex: 1;
    padding: 1rem;
    background: none;
    border: none;
    font-size: 1rem;
    cursor: pointer;
    color: var(--text-color);
  }

  .nav-button.active {
    background: var(--accent-color);
    color: var(--accent-text-color);
  }
</style>