import { searchSongs } from '@/api'
import type { SongSummary } from '@/types'

// The queue/history endpoints embed SongSummary with id hardcoded to 0
// (backend bug), so covers can't be fetched from those ids. Build a local
// title|artist → SongSummary index from the songs endpoint and resolve
// queue rows against it.

const KEY_SEP = '\u0000'

function keyOf(s: Pick<SongSummary, 'title' | 'artist'>): string {
  return `${s.title}${KEY_SEP}${s.artist}`
}

let cache: Map<string, SongSummary> | null = null
let loading: Promise<Map<string, SongSummary>> | null = null

async function fetchAll(): Promise<Map<string, SongSummary>> {
  const map = new Map<string, SongSummary>()
  let offset = 0
  for (;;) {
    const res = await searchSongs('', 200, offset)
    for (const song of res.data) map.set(keyOf(song), song)
    if (offset + res.data.length >= res.total || res.data.length === 0) break
    offset += res.data.length
    if (offset >= 6000) break // safety cap
  }
  return map
}

/** Full song index; cached in memory (no stale > 5 min). */
export function loadSongIndex(): Promise<Map<string, SongSummary>> {
  if (cache) return Promise.resolve(cache)
  if (loading) return loading
  loading = fetchAll()
    .then((map) => {
      cache = map
      return map
    })
    .finally(() => {
      loading = null
    })
  return loading
}

export function invalidateSongIndex() {
  cache = null
}

/** Resolve a possibly id-stripped summary to a real one, if known. */
export function resolveSong(song: SongSummary | null | undefined): SongSummary | null {
  if (!song) return null
  if (song.id > 0) return song
  return cache?.get(keyOf(song)) ?? null
}
