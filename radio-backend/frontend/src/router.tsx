import { createBrowserRouter } from 'react-router-dom'
import App from './App'
import NowPlayingPage from './pages/NowPlayingPage'
import PlayerPage from './pages/PlayerPage'
import QueuePage from './pages/QueuePage'
import UpNextPage from './pages/UpNextPage'
import LibraryPage from './pages/LibraryPage'
import SettingsPage from './pages/SettingsPage'
import AdminPage from './pages/AdminPage'
import ListenPage from './pages/ListenPage'

export const router = createBrowserRouter(
  [
    {
      path: '/',
      element: <App />,
      children: [
        { index: true, element: <NowPlayingPage /> },
        { path: 'player', element: <PlayerPage /> },
        { path: 'queue', element: <QueuePage /> },
        { path: 'up-next', element: <UpNextPage /> },
        { path: 'library', element: <LibraryPage /> },
        { path: 'settings', element: <SettingsPage /> },
        { path: 'admin', element: <AdminPage /> },
        { path: 'listen', element: <ListenPage /> },
      ],
    },
  ],
  { basename: import.meta.env.BASE_URL.replace(/\/$/, '') },
)
