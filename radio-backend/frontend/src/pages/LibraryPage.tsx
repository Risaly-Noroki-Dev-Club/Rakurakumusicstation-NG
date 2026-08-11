import { Tabs, TabsList, TabsTrigger, TabsContent } from '@appica/ui-react/tabs'
import { SongSearch } from '@/components/library/SongSearch'
import { FavoritesTab } from '@/components/library/FavoritesTab'

/** 曲库页：歌曲搜索 / 本地收藏。 */
export default function LibraryPage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-4 px-4 py-4 sm:px-6 sm:py-6">
      <Tabs defaultValue="songs" variant="line">
        <TabsList className="w-full">
          <TabsTrigger value="songs" className="flex-1">
            歌曲
          </TabsTrigger>
          <TabsTrigger value="favorites" className="flex-1">
            收藏
          </TabsTrigger>
        </TabsList>
        <TabsContent value="songs" keepMounted className="pt-4">
          <SongSearch />
        </TabsContent>
        <TabsContent value="favorites" keepMounted className="pt-4">
          <FavoritesTab />
        </TabsContent>
      </Tabs>
    </div>
  )
}
