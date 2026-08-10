import { Badge } from '@appica/ui-react/badge'
import { Carousel, CarouselContent, CarouselSlide, CarouselPagination } from '@appica/ui-react/carousel'
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@appica/ui-react/collapsible'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { Avatar, AvatarFallback, AvatarGroup } from '@appica/ui-react/avatar'
import { ChevronDown, FileMusic, Headphones, Playlist } from '@appica/icons-react'
import { SongArtwork } from '@/components/SongArtwork'
import { useStore, type Playback } from '@/store'
import { usePlaybackClock } from '@/hooks/usePlaybackClock'
import { LyricsPanel } from '@/components/player/LyricsPanel'
import { QueueList } from '@/components/queue/QueueList'

/**
 * 播放器主栏：封面 banner + 歌名，下方直接是歌词（原控制栏位置）。
 * 播放控制（含管理员切歌）移入底部 MiniPlayer。
 */
function PlayerPane({ playback, position }: { playback: Playback | null; position: number }) {
  const station = useStore((s) => s.station)
  const onAir = playback !== null && playback.title.length > 0
  const artwork = onAir ? (playback?.coverUrl ?? undefined) : (station?.icon_url ?? undefined)

  return (
    <div className="flex min-w-0 flex-col items-center">
      <SongArtwork
        hasCover={artwork !== undefined}
        coverSrc={artwork}
        alt={onAir ? `${playback.title} 封面` : station?.name ? `${station.name} 图标` : ''}
        size="2xl"
        className="size-52 sm:size-60 lg:size-72"
      />

      <div className="mt-5 w-full min-w-0 text-center">
        <h1
          className="text-foreground-intense truncate text-3xl font-bold sm:text-4xl"
          title={onAir ? playback.title : station?.name}
        >
          {onAir ? playback.title : station?.name ?? 'Rakuraku Music Station'}
        </h1>
        <p className="text-foreground-muted mt-1 truncate text-base">
          {onAir ? playback.artist || '\u00a0' : station?.subtitle || '\u00a0'}
        </p>
      </div>

      {/* 歌词占据原控制栏的位置 */}
      <div className="mt-8 w-full max-w-xl">
        <h2 className="text-foreground-intense mb-3 flex items-center gap-1.5 text-sm font-semibold">
          <FileMusic className="text-foreground-muted size-4" aria-hidden="true" />
          歌词
        </h2>
        <ScrollArea className="max-h-[45vh] min-h-0 w-full" scrollShadow scrollbarVisibility="auto">
          <LyricsPanel playback={playback} positionMs={position} className="max-h-none overflow-visible" />
        </ScrollArea>
      </div>
    </div>
  )
}

/** 点歌队列（xl 起右栏）。 */
function QueuePane({ count }: { count: number }) {
  return (
    <aside aria-label="点歌队列" className="min-w-0 xl:border-border-muted xl:border-l xl:ps-8">
      <div className="mb-3 flex items-center gap-2">
        <Playlist className="text-foreground-muted size-4" aria-hidden="true" />
        <h2 className="text-foreground-intense text-sm font-semibold">点歌队列</h2>
        <Badge variant="soft" size="xs" aria-label={`队列中共 ${count} 首`}>
          {count}
        </Badge>
      </div>
      <QueueList />
    </aside>
  )
}

/**
 * 首页：封面 banner + 歌词为主角；播放控制全部在底部 MiniPlayer。
 */
export default function PlayerPage() {
  const playback = useStore((s) => s.playback)
  const names = useStore((s) => s.listeners.names)
  const count = useStore((s) => s.listeners.count)
  const displayName = useStore((s) => s.auth?.display_name ?? '匿名')
  const queueCount = useStore((s) => s.queue.length)
  const position = usePlaybackClock(playback)
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
          {displayName !== '匿名' && (
            <span className="text-foreground-muted ms-auto truncate text-xs">{displayName}</span>
          )}
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

      {/* 移动端（<lg）：播放器（含歌词）/ 队列 两卡滑动 */}
      <div className="lg:hidden">
        <Carousel align="start" slidesToScroll={1}>
          <CarouselContent>
            <CarouselSlide className="basis-full">
              <PlayerPane playback={playback} position={position} />
            </CarouselSlide>
            <CarouselSlide className="basis-full">
              <QueuePane count={queueCount} />
            </CarouselSlide>
          </CarouselContent>
          <CarouselPagination className="mt-4 justify-center" />
        </Carousel>
      </div>

      {/* 桌面（lg+）：主栏 + xl 右队列 */}
      <div className="hidden gap-10 lg:block xl:grid xl:grid-cols-[minmax(0,1fr)_320px] xl:gap-12">
        <PlayerPane playback={playback} position={position} />
        <QueuePane count={queueCount} />
      </div>
    </div>
  )
}
