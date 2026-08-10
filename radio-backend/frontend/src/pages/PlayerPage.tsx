import { Link } from 'react-router-dom'
import { Progress } from '@appica/ui-react/progress'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { Thumbnail } from '@appica/ui-react/thumbnail'
import { ArrowLeft, Music, Radio } from '@appica/icons-react'
import { useStore } from '@/store'
import { usePlaybackClock } from '@/hooks/usePlaybackClock'
import { formatTime } from '@/lib/format'
import { LyricsPanel } from '@/components/player/LyricsPanel'
import { PlayerControls } from '@/components/player/PlayerControls'

/** Full-size player: artwork + controls + progress, with lyrics beside it. */
export default function PlayerPage() {
  const station = useStore((s) => s.station)
  const playback = useStore((s) => s.playback)
  const position = usePlaybackClock(playback)

  const onAir = playback !== null && playback.title.length > 0
  const artwork = onAir ? (playback?.coverUrl ?? undefined) : (station?.icon_url ?? undefined)
  const ArtIcon = onAir ? Music : Radio
  const duration = playback?.durationMs ?? 0
  const pct = duration > 0 ? Math.min(100, (position / duration) * 100) : 0

  return (
    <div className="mx-auto w-full max-w-6xl px-4 py-6">
      <Link
        to="/"
        className="text-foreground-muted hover:text-foreground-intense inline-flex items-center gap-1 text-sm transition-colors"
      >
        <ArrowLeft className="size-4" />
        返回正在播放
      </Link>

      <div className="mt-6 grid gap-10 lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)] lg:gap-12">
        {/* Left: artwork + title + controls + progress */}
        <div className="flex min-w-0 flex-col items-center gap-5">
          <Thumbnail
            size="2xl"
            shape="rounded"
            variant={artwork ? 'image' : 'icon-soft'}
            src={artwork}
            alt={onAir ? `${playback.title} 封面` : station?.name ? `${station.name} 图标` : ''}
            className="size-56 md:size-72 shadow-lg shadow-black/10"
          >
            <ArtIcon />
          </Thumbnail>

          <div className="w-full min-w-0 text-center">
            <h1 className="text-foreground-intense text-[clamp(1.5rem,4vw,2.25rem)] leading-tight font-bold break-words">
              {onAir ? playback.title : station?.name ?? 'Rakuraku Music Station'}
            </h1>
            <p className="text-foreground-muted mt-1.5 truncate text-base">
              {onAir ? playback.artist || '\u00a0' : station?.subtitle || '\u00a0'}
            </p>
          </div>

          <PlayerControls className="w-full" />

          <div className="w-full min-w-0">
            <Progress value={pct} className="w-full" aria-label="播放进度" />
            <div className="text-foreground-subtle mt-1.5 flex w-full justify-between text-xs tabular-nums">
              <span>{formatTime(position)}</span>
              <span>{formatTime(duration)}</span>
            </div>
          </div>
        </div>

        {/* Right: lyrics in a bounded ScrollArea */}
        <div className="min-w-0">
          <h2 className="text-foreground-intense mb-3 text-base font-semibold">歌词</h2>
          <ScrollArea
            className="max-h-[55vh] min-h-0 w-full lg:max-h-[70vh]"
            scrollShadow
            scrollbarVisibility="auto"
          >
            <LyricsPanel playback={playback} positionMs={position} className="max-h-none overflow-visible" />
          </ScrollArea>
        </div>
      </div>
    </div>
  )
}
