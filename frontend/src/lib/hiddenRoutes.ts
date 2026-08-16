// Hidden-route preferences live in localStorage, one list per stop.
//
// Entries are keyed "type-route" so a stop served by both bus 1 and tram 1 can
// hide one without the other. Entries written before per-type rows existed are
// bare route numbers; those still hide every type sharing the number, and are
// dropped the first time that route is toggled.

export function hiddenKey(type: string, route: string): string {
  return `${type}-${route}`;
}

function legacyKey(key: string): string {
  return key.slice(key.indexOf('-') + 1);
}

export function isHiddenRoute(hidden: string[], type: string, route: string): boolean {
  return hidden.includes(hiddenKey(type, route)) || hidden.includes(route);
}

export function toggleHidden(hidden: string[], key: string): string[] {
  const remaining = hidden.filter(entry => entry !== key && entry !== legacyKey(key));
  return remaining.length === hidden.length ? [...hidden, key] : remaining;
}
