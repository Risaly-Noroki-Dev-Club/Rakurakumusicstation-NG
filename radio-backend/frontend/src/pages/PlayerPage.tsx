import { Badge } from '@appica/ui-react/badge'
import { Carousel, CarouselContent, CarouselSlide, CarouselPagination } from '@appica/ui-react/carousel'
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@appica/ui-react/collapsible'
import { Progress } from '@appica/ui-react/progress'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { Avatar, AvatarFallback, AvatarGroup } from '@appica/ui-react/avatar'
import { ChevronDown, FileMusic, Headphones, Playlist } from '@appica/icons-react'
import { SongArtwork } from '@/components/SongArtwork'
import { useStore, type Playback } from '@/store'
import { usePlaybackClock } from '@/hooks/usePlaybackClock'
import { formatTime } from '@/lib/format'
import { LyricsPanel } from '@/components/player/LyricsPanel'
import { PlayerControls } from '@/components/player/PlayerControls'
import { QueueList } from '@/components/queue/QueueList'

/** 正在播放卡：封面 + 标题 + 控制 + 进度。 */
function NowPlayingPane({ playback, position }: { playback: Playback | null; position: number }) {
  const station = useStore((s) => s.station)
  const onAir = playback !== null && playback.title.length > 0
  const artwork = onAir ? (playback?.coverUrl ?? undefined) : (station?.icon_url ?? undefined)
  const duration = playback?.durationMs ?? 0
  const pct = duration > 0 ? Math.min(100, (position / duration) * 100) : 0

  return (
    <div className="flex min-w-0 flex-col items-center gap-5">
      <SongArtwork
        hasCover={artwork !== undefined}
        coverSrc={artwork}
        alt={onAir ? `${playback.title} 封面` : station?.name ? `${station.name} 图标` : ''}
        size="2xl"
        className="size-52 shadow-lg shadow-black/10 sm:size-64 lg:size-72"
      />

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
  )
}

/** 歌词卡。 */
function LyricsPane({ playback, position, tall }: { playback: Playback | null; position: number; tall: boolean }) {
  return (
    <div className="min-w-0">
      <h2 className="text-foreground-intense mb-3 flex items-center gap-1.5 text-base font-semibold">
        <FileMusic className="text-primary size-4.5" aria-hidden="true" />
        歌词
      </h2>
      <ScrollArea
        className={tall ? 'max-h-[70vh] min-h-0 w-full' : 'max-h-[50vh] min-h-0 w-full'}
        scrollShadow
        scrollbarVisibility="auto"
      >
        <LyricsPanel playback={playback} positionMs={position} className="max-h-none overflow-visible" />
      </ScrollArea>
    </div>
  )
}

/** 点歌队列卡。 */
function QueuePane({ count }: { count: number }) {
  return (
    <aside aria-label="点歌队列" className="min-w-0">
      <div className="mb-3 flex items-center gap-2">
        <Playlist className="text-primary size-4.5" aria-hidden="true" />
        <h2 className="text-foreground-intense text-sm font-semibold">点歌队列</h2>
        <Badge variant="secondary" size="sm" aria-label={`队列中共 ${count} 首`}>
          {count}
        </Badge>
      </div>
      <QueueList />
    </aside>
  )
}

/**
 * Home page: the full player merged with the "listen together" presence.
 * Mobile: the three cards (player / lyrics / queue) are swipeable carousel
 * pages with a dot indicator; lg+: a multi-column grid.
 */
export default function PlayerPage() {
  const playback = useStore((s) => s.playback)
  const names = useStore((s) => s.listeners.names)
  const count = useStore((s) => s.listeners.count)
  const displayName = useStore((s) => s.auth?.display_name ?? '')
  const queueCount = useStore((s) => s.queue.length)
  const position = usePlaybackClock(playback)
  // The server includes this device in `names`; show only *other* listeners,
  // since "我" has its own row at the bottom of the expanded list.
  const others = names.filter((name) => name !== displayName)

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
          {displayName && <span className="text-foreground-muted ms-auto truncate text-xs">{displayName}</span>}
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
              <span className="text-foreground-intense min-w-0 truncate text-sm font-medium">{displayName || '我'}</span>
              <span className="text-foreground-muted ms-auto shrink-0 text-xs">我 · 当前设备</span>
            </li>
          </ul>
        </CollapsibleContent>
      </Collapsible>

      {/* Mobile (<lg): swipeable card pages */}
      <div className="lg:hidden">
        <Carousel align="start" slidesToScroll={1}>
          <CarouselContent>
            <CarouselSlide className="basis-full">
              <NowPlayingPane playback={playback} position={position} />
            </CarouselSlide>
            <CarouselSlide className="basis-full">
              <LyricsPane playback={playback} position={position} tall={false} />
            </CarouselSlide>
            <CarouselSlide className="basis-full">
              <QueuePane count={queueCount} />
            </CarouselSlide>
          </CarouselContent>
          <CarouselPagination className="mt-5 justify-center" />
        </Carousel>
      </div>

      {/* Desktop (lg+): multi-column grid */}
      <div className="hidden gap-10 lg:grid lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)] lg:gap-12 xl:grid-cols-[minmax(0,4fr)_minmax(0,5fr)_320px]">
        <NowPlayingPane playback={playback} position={position} />
        <LyricsPane playback={playback} position={position} tall />
        <QueuePane count={queueCount} />
      </div>
    </div>
  )
}
