import { Tabs, TabsList, TabsTrigger, TabsContent } from '@appica/ui-react/tabs'
import { SongSearch } from '@/components/library/SongSearch'
import { PlaylistsTab } from '@/components/library/PlaylistsTab'
import { FavoritesTab } from '@/components/library/FavoritesTab'

/** 曲库页：歌曲搜索 / 播放列表 / 收藏。 */
export default function LibraryPage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-4 px-4 py-4 sm:px-6 sm:py-6">
      <header className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold sm:text-2xl">曲库</h1>
        <p className="text-sm text-foreground-muted">搜索歌曲、管理播放列表与收藏</p>
      </header>
      <Tabs defaultValue="songs" variant="line">
        <TabsList className="w-full">
          <TabsTrigger value="songs" className="flex-1">
            歌曲
          </TabsTrigger>
          <TabsTrigger value="playlists" className="flex-1">
            播放列表
          </TabsTrigger>
          <TabsTrigger value="favorites" className="flex-1">
            收藏
          </TabsTrigger>
        </TabsList>
        <TabsContent value="songs" keepMounted className="pt-4">
          <SongSearch />
        </TabsContent>
        <TabsContent value="playlists" keepMounted className="pt-4">
          <PlaylistsTab />
        </TabsContent>
        <TabsContent value="favorites" keepMounted className="pt-4">
          <FavoritesTab />
        </TabsContent>
      </Tabs>
    </div>
  )
}
