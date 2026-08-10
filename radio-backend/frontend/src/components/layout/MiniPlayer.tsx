import { useNavigate } from 'react-router-dom'
import { Button } from '@appica/ui-react/button'
import { Progress } from '@appica/ui-react/progress'
import { PlayerPause, PlayerPlay, Maximize } from '@appica/icons-react'
import { SongArtwork } from '@/components/SongArtwork'
import { useStore } from '@/store'
import { usePlaybackClock } from '@/hooks/usePlaybackClock'
import { formatTime } from '@/lib/format'
import { isAudioPlaying, pauseAudio, resumeAudio } from '@/audio/streamAudio'

/** Docked bottom player bar, visible whenever a track is on air. */
export function MiniPlayer() {
  const playback = useStore((s) => s.playback)
  const navigate = useNavigate()
  const position = usePlaybackClock(playback)
  const playing = isAudioPlaying()

  if (!playback || !playback.title) return null

  const duration = playback.durationMs || 0
  const pct = duration > 0 ? Math.min(100, (position / duration) * 100) : 0

  return (
    <div className="border-border-muted bg-background/95 sticky bottom-0 z-40 border-t backdrop-blur">
      <div className="mx-auto flex h-16 w-full max-w-6xl items-center gap-3 px-4">
        <SongArtwork
          hasCover={playback.coverUrl !== null}
          coverSrc={playback.coverUrl ?? undefined}
          size="sm"
        />
        <div className="min-w-0 flex-1">
          <p className="text-foreground-intense truncate text-sm font-medium">{playback.title}</p>
          <p className="text-foreground-muted truncate text-xs">{playback.artist || '\u00a0'}</p>
        </div>
        <div className="hidden w-44 items-center gap-2 sm:flex">
          <Progress value={pct} className="min-w-0 flex-1" aria-label="播放进度" />
          <span className="text-foreground-muted shrink-0 text-xs tabular-nums">{formatTime(position)}</span>
        </div>
        <Button
          variant="outline"
          size="icon-md"
          aria-label={playing ? '暂停播放' : '继续播放'}
          onClick={() => (playing ? pauseAudio() : resumeAudio())}
        >
          {playing ? <PlayerPause /> : <PlayerPlay />}
        </Button>
        <Button
          variant="ghost"
          size="icon-md"
          aria-label="打开播放器"
          onClick={() => navigate('/player')}
        >
          <Maximize />
        </Button>
      </div>
    </div>
  )
}
