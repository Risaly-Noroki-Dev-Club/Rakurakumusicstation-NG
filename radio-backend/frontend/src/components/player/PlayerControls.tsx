import { Button } from '@appica/ui-react/button'
import { PlayerPause, PlayerPlay, PlayerSkipBack, PlayerSkipForward } from '@appica/icons-react'
import { useStore } from '@/store'
import { isAudioPlaying, pauseAudio, resumeAudio } from '@/audio/streamAudio'
import { adminSkipNext, adminSkipPrev } from '@/api'
import { cn } from '@/lib/cn'

export interface PlayerControlsProps {
  className?: string
}

function StatusDot({ color, pulsing }: { color: string; pulsing: boolean }) {
  if (!pulsing) return <span aria-hidden="true" className={cn('inline-block size-2 rounded-full', color)} />
  return (
    <span aria-hidden="true" className="relative flex size-2">
      <span className={cn('absolute inline-flex size-full animate-ping rounded-full opacity-60', color)} />
      <span className={cn('relative inline-flex size-2 rounded-full', color)} />
    </span>
  )
}

/**
 * Play/pause + (admin-only) skip controls, plus a live status readout
 * (直播中 / 暂停 / 已停止). Reads auth + playback from the store.
 */
export function PlayerControls({ className }: PlayerControlsProps) {
  const status = useStore((s) => s.playback?.status)
  const audioPaused = useStore((s) => s.audioPaused)
  const isAdmin = useStore((s) => s.auth?.role === 'admin')
  const canPlay = useStore((s) => s.playback !== null && s.playback.status !== 'stopped' && !!s.playback.streamUrl)
  const playing = isAudioPlaying()

  const togglePlayback = () => {
    if (playing) pauseAudio()
    else resumeAudio()
  }

  const skip = async (dir: 'prev' | 'next') => {
    try {
      if (dir === 'prev') await adminSkipPrev()
      else await adminSkipNext()
    } catch (e) {
      useStore.getState().addToast(e instanceof Error ? e.message : '操作失败', 'error')
    }
  }

  const live = status === 'playing' || status === 'crossfading'
  let statusText: string
  let dotColor: string
  let pulsing: boolean
  if (!live) {
    statusText = '已停止'
    dotColor = 'bg-foreground-subtle'
    pulsing = false
  } else if (audioPaused) {
    statusText = '暂停'
    dotColor = 'bg-warning'
    pulsing = false
  } else {
    statusText = '直播中'
    dotColor = 'bg-error'
    pulsing = true
  }

  return (
    <div className={cn('flex flex-col items-center gap-3', className)}>
      <div className="flex items-center gap-3">
        {isAdmin && (
          <Button
            variant="outline"
            size="icon-md"
            aria-label="上一首"
            disabled={!canPlay}
            onClick={() => void skip('prev')}
          >
            <PlayerSkipBack />
          </Button>
        )}
        <Button
          variant="primary"
          size="icon-lg"
          className="rounded-full"
          aria-label={playing ? '暂停播放' : '开始播放'}
          disabled={!canPlay}
          onClick={togglePlayback}
        >
          {playing ? <PlayerPause /> : <PlayerPlay />}
        </Button>
        {isAdmin && (
          <Button
            variant="outline"
            size="icon-md"
            aria-label="下一首"
            disabled={!canPlay}
            onClick={() => void skip('next')}
          >
            <PlayerSkipForward />
          </Button>
        )}
      </div>
      <p className="flex items-center gap-2 text-sm">
        <StatusDot color={dotColor} pulsing={pulsing} />
        <span className="text-foreground-muted">{statusText}</span>
      </p>
    </div>
  )
}
