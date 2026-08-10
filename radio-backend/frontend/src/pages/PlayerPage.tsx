import { Badge } from '@appica/ui-react/badge'
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@appica/ui-react/collapsible'
import { Progress } from '@appica/ui-react/progress'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { Thumbnail } from '@appica/ui-react/thumbnail'
import { Avatar, AvatarFallback, AvatarGroup } from '@appica/ui-react/avatar'
import { ChevronDown, FileMusic, Headphones, Music, Playlist, Radio } from '@appica/icons-react'
import { useStore } from '@/store'
import { usePlaybackClock } from '@/hooks/usePlaybackClock'
import { formatTime } from '@/lib/format'
import { LyricsPanel } from '@/components/player/LyricsPanel'
import { PlayerControls } from '@/components/player/PlayerControls'
import { QueueList } from '@/components/queue/QueueList'

/**
 * Home page: the full player (artwork + controls + lyrics) merged with the
 * "listen together" presence — a listeners strip on top and the shared queue
 * (with admin management) as a side panel from xl up.
 */
export default function PlayerPage() {
  const station = useStore((s) => s.station)
  const playback = useStore((s) => s.playback)
  const names = useStore((s) => s.listeners.names)
  const count = useStore((s) => s.listeners.count)
  const displayName = useStore((s) => s.auth?.display_name ?? '匿名')
  // The server includes this device in `names`; show only *other* listeners,
  // since "我" has its own row at the bottom of the expanded list.
  const others = names.filter((name) => name !== displayName)
  const queueCount = useStore((s) => s.queue.length)
  const position = usePlaybackClock(playback)

  const onAir = playback !== null && playback.title.length > 0
  const artwork = onAir ? (playback?.coverUrl ?? undefined) : (station?.icon_url ?? undefined)
  const ArtIcon = onAir ? Music : Radio
  const duration = playback?.durationMs ?? 0
  const pct = duration > 0 ? Math.min(100, (position / duration) * 100) : 0

  return (
    <div className="mx-auto w-full max-w-6xl px-4 py-6">
      {/* Listeners strip (WS listeners_update presence) — click to expand */}
      <Collapsible className="border-border-muted bg-background-subtle mb-6 overflow-hidden rounded-xl border">
        <CollapsibleTrigger className="group flex w-full flex-wrap items-center gap-x-3 gap-y-2 px-4 py-3 text-start">
          <Headphones className="text-primary size-4.5 shrink-0" aria-hidden="true" />
          <span className="text-foreground-intense text-sm font-semibold">正在收听</span>
          <Badge variant="secondary" size="sm" aria-label={`共 ${count} 人在线`}>
            {count}
          </Badge>
          {others.length > 0 ? (
            <AvatarGroup size="sm" className="flex-wrap">
              {others.map((name, i) => (
                <Avatar key={`${name}-${i}`} title={name}>
                  <AvatarFallback>{name.trim().charAt(0).toUpperCase() || '?'}</AvatarFallback>
                </Avatar>
              ))}
            </AvatarGroup>
          ) : (
            <span className="text-foreground-muted text-xs">还没有其他听众，快邀请朋友一起来吧</span>
          )}
          <span className="text-foreground-muted ms-auto truncate text-xs">{displayName}</span>
          <ChevronDown
            className="text-foreground-muted size-4 shrink-0 transition-transform duration-200 group-data-panel-open:rotate-180 motion-reduce:transition-none"
            aria-hidden="true"
          />
        </CollapsibleTrigger>
        <CollapsibleContent>
          <ul aria-label="在线听众" className="divide-border-muted divide-y border-t px-4 py-1">
            {others.map((name, i) => (
              <li key={`${name}-${i}`} className="flex items-center gap-3 py-2.5">
                <Avatar size="sm">
                  <AvatarFallback>{name.trim().charAt(0).toUpperCase() || '?'}</AvatarFallback>
                </Avatar>
                <span className="text-foreground-intense min-w-0 truncate text-sm font-medium">{name}</span>
                <span className="text-success ms-auto flex shrink-0 items-center gap-1.5 text-xs">
                  <span aria-hidden="true" className="relative flex size-1.5">
                    <span className="absolute inline-flex size-full animate-ping rounded-full bg-current opacity-60" />
                    <span className="relative inline-flex size-1.5 rounded-full bg-current" />
                  </span>
                  在线
                </span>
              </li>
            ))}
            <li className="flex items-center gap-3 py-2.5">
              <Avatar size="sm">
                <AvatarFallback className="bg-primary text-primary-foreground">
                  {displayName.trim().charAt(0).toUpperCase() || '我'}
                </AvatarFallback>
              </Avatar>
              <span className="text-foreground-intense min-w-0 truncate text-sm font-medium">{displayName}</span>
              <span className="text-foreground-muted ms-auto shrink-0 text-xs">我 · 当前设备</span>
            </li>
          </ul>
        </CollapsibleContent>
      </Collapsible>

      <div className="grid gap-10 lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)] lg:gap-12 xl:grid-cols-[minmax(0,4fr)_minmax(0,5fr)_320px]">
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

        {/* Center: lyrics in a bounded ScrollArea */}
        <div className="min-w-0">
          <h2 className="text-foreground-intense mb-3 flex items-center gap-1.5 text-base font-semibold">
            <FileMusic className="text-primary size-4.5" aria-hidden="true" />
            歌词
          </h2>
          <ScrollArea
            className="max-h-[40vh] min-h-0 w-full lg:max-h-[55vh] xl:max-h-[70vh]"
            scrollShadow
            scrollbarVisibility="auto"
          >
            <LyricsPanel playback={playback} positionMs={position} className="max-h-none overflow-visible" />
          </ScrollArea>
        </div>

        {/* Right (xl+): shared request queue with admin management */}
        <aside aria-label="点歌队列" className="min-w-0 xl:border-border-muted xl:border-l xl:ps-6">
          <div className="mb-3 flex items-center gap-2">
            <Playlist className="text-primary size-4.5" aria-hidden="true" />
            <h2 className="text-foreground-intense text-sm font-semibold">点歌队列</h2>
            <Badge variant="secondary" size="sm" aria-label={`队列中共 ${queueCount} 首`}>
              {queueCount}
            </Badge>
          </div>
          <QueueList />
        </aside>
      </div>
    </div>
  )
}
