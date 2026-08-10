import { Button } from '@appica/ui-react/button'
import { Progress } from '@appica/ui-react/progress'
import { Thumbnail } from '@appica/ui-react/thumbnail'
import { Music, PlayerPause, PlayerPlay } from '@appica/icons-react'
import { useStore } from '@/store'
import { usePlaybackClock } from '@/hooks/usePlaybackClock'
import { formatTime } from '@/lib/format'
import { isAudioPlaying, pauseAudio, resumeAudio } from '@/audio/streamAudio'

/** 一起听 — current track card with local transport controls. */
export function LtPlayerCard() {
  const playback = useStore((s) => s.playback)
  const position = usePlaybackClock(playback)
  const playing = isAudioPlaying()

  if (!playback || !playback.title) {
    return (
      <div className="border-border-muted flex items-center justify-center gap-3 rounded-xl border p-6">
        <Music className="text-foreground-subtle" />
        <p className="text-foreground-muted text-sm">电台还没开播，等待第一首歌…</p>
      </div>
    )
  }

  const pct = playback.durationMs > 0 ? Math.min(100, (position / playback.durationMs) * 100) : 0

  return (
    <div className="border-border-muted bg-background-subtle flex items-center gap-4 rounded-xl border p-4">
      <Thumbnail
        size="lg"
        shape="rounded"
        variant={playback.coverUrl ? 'image' : 'icon-soft'}
        src={playback.coverUrl ?? undefined}
        alt=""
      >
        <Music />
      </Thumbnail>
      <div className="min-w-0 flex-1">
        <p className="text-foreground-intense truncate text-base font-semibold">{playback.title}</p>
        <p className="text-foreground-muted truncate text-sm">{playback.artist || '\u00a0'}</p>
        <div className="mt-2 flex items-center gap-2">
          <Progress value={pct} className="min-w-0 flex-1" aria-label="播放进度" />
          <span className="text-foreground-muted shrink-0 text-xs tabular-nums">{formatTime(position)}</span>
        </div>
      </div>
      <Button
        variant="primary"
        size="icon-lg"
        aria-label={playing ? '暂停播放' : '继续播放'}
        onClick={() => (playing ? pauseAudio() : resumeAudio())}
        className="shrink-0 rounded-full"
      >
        {playing ? <PlayerPause /> : <PlayerPlay />}
      </Button>
    </div>
  )
}
