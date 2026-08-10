// Remember covers that loaded successfully so we can skip the preload
// round-trip on remounts (the browser HTTP cache still serves the bytes;
// this avoids the wait and the failed-image flash for slow covers).

const KEY = 'rakuraku.cover-ok'
const MAX_ENTRIES = 500

let known = new Set<string>()

function load() {
  try {
    const raw = sessionStorage.getItem(KEY)
    if (raw) known = new Set(JSON.parse(raw) as string[])
  } catch {
    known = new Set()
  }
}

load()

export function isCoverKnownGood(url: string): boolean {
  return known.has(url)
}

export function rememberCoverSuccess(url: string) {
  known.add(url)
  try {
    const list = [...known]
    if (list.length > MAX_ENTRIES) list.splice(0, list.length - MAX_ENTRIES)
    sessionStorage.setItem(KEY, JSON.stringify(list))
  } catch {
    // storage full/unavailable — in-memory set still works this session
  }
}
