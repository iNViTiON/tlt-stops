import { writable } from 'svelte/store';

export const currentTime = writable(Date.now());

setInterval(() => {
  currentTime.set(Date.now());
}, 1000);