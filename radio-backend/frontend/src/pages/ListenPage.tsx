import { LtSidebar } from '@/components/lt/LtSidebar'
import { LtPlayerCard } from '@/components/lt/LtPlayerCard'
import { LtLyricsPanel } from '@/components/lt/LtLyricsPanel'
import { LtQueuePanel } from '@/components/lt/LtQueuePanel'

/**
 * 一起听 — shared-listening presence view.
 *
 * Backend has no rooms: this is presence (WS listeners_update) + shared
 * playback + shared queue. The warm "together" accent is applied by
 * overriding the --accent CSS variable on this subtree (index.css derives
 * the primary tokens from it).
 */
export default function ListenPage() {
  return (
    <div
      style={{ ['--accent' as string]: '#e2725b' }}
      className="mx-auto flex h-full w-full max-w-6xl flex-col md:flex-row"
    >
      <LtSidebar />
      <div className="flex min-h-0 flex-1 flex-col gap-4 p-4">
        <LtPlayerCard />
        <LtLyricsPanel />
      </div>
      <LtQueuePanel />
    </div>
  )
}
