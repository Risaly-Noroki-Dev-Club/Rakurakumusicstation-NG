import { Link } from 'react-router-dom'
import { Badge } from '@appica/ui-react/badge'
import { Button } from '@appica/ui-react/button'
import { Thumbnail } from '@appica/ui-react/thumbnail'
import { Music, Radio } from '@appica/icons-react'
import { useStore } from '@/store'
import { usePlaybackClock } from '@/hooks/usePlaybackClock'
import { formatTime } from '@/lib/format'
import { LyricsPanel } from '@/components/player/LyricsPanel'
import { PlayerControls } from '@/components/player/PlayerControls'

/** Radio hero: album art, station identity, LIVE badge, now-playing info and lyrics. */
export default function NowPlayingPage() {
  const station = useStore((s) => s.station)
  const playback = useStore((s) => s.playback)
  const position = usePlaybackClock(playback)

  const onAir = playback !== null && playback.title.length > 0
  const live = playback?.status === 'playing'
  const artwork = onAir ? (playback?.coverUrl ?? undefined) : (station?.icon_url ?? undefined)
  const ArtIcon = onAir ? Music : Radio

  return (
    <section className="mx-auto flex w-full max-w-2xl flex-col items-center px-4 pt-8 pb-12 text-center">
      <Thumbnail
        size="2xl"
        shape="rounded"
        variant={artwork ? 'image' : 'icon-soft'}
        src={artwork}
        alt={onAir ? `${playback.title} 封面` : station?.name ? `${station.name} 图标` : ''}
        className="size-48 md:size-60 shadow-lg shadow-black/10"
      >
        <ArtIcon />
      </Thumbnail>

      {live && (
        <Badge variant="error" size="sm" className="mt-5 gap-1.5">
          <span aria-hidden="true" className="relative flex size-2">
            <span className="absolute inline-flex size-full animate-ping rounded-full bg-current opacity-60" />
            <span className="relative inline-flex size-2 rounded-full bg-current" />
          </span>
          LIVE
        </Badge>
      )}

      <div className="mt-4 w-full min-w-0">
        {onAir ? (
          <>
            <h1 className="text-foreground-intense text-[clamp(1.875rem,5vw,2.5rem)] leading-tight font-bold break-words">
              {playback.title}
            </h1>
            <p className="text-foreground-muted mt-2 truncate text-lg">{playback.artist || '\u00a0'}</p>
          </>
        ) : (
          <>
            <h1 className="text-foreground-intense text-[clamp(1.875rem,5vw,2.5rem)] leading-tight font-bold break-words">
              {station?.name ?? 'Rakuraku Music Station'}
            </h1>
            <p className="text-foreground-muted mt-2 truncate text-lg">{station?.subtitle || '\u00a0'}</p>
          </>
        )}
      </div>

      {onAir && (
        <p className="text-foreground-subtle mt-3 text-sm tabular-nums">
          {formatTime(position)} / {formatTime(playback.durationMs)}
        </p>
      )}

      {onAir ? (
        <PlayerControls className="mt-6" />
      ) : (
        <div className="mt-6 flex flex-col items-center gap-1.5">
          <p className="text-foreground-muted text-sm">等待开播</p>
          <p className="text-foreground-subtle text-xs">电台还没有播放内容，去曲库点一首歌吧</p>
          <Button render={<Link to="/library" />} variant="primary" size="lg" className="mt-3">
            <Music data-icon="start" />
            点歌
          </Button>
        </div>
      )}

      {onAir && (
        <div className="mt-8 w-full max-w-xl">
          <LyricsPanel playback={playback} positionMs={position} />
        </div>
      )}
    </section>
  )
}
