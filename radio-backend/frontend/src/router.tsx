import { createBrowserRouter } from 'react-router-dom'
import App from './App'
import PlayerPage from './pages/PlayerPage'
import LibraryPage from './pages/LibraryPage'
import SettingsPage from './pages/SettingsPage'
import AdminPage from './pages/AdminPage'

/**
 * The player is the home page and also hosts the "listen together" presence
 * (listeners strip + shared queue), so there is no separate listen route.
 */
export const router = createBrowserRouter(
  [
    {
      path: '/',
      element: <App />,
      children: [
        { index: true, element: <PlayerPage /> },
        { path: 'player', element: <PlayerPage /> },
        { path: 'library', element: <LibraryPage /> },
        { path: 'settings', element: <SettingsPage /> },
        { path: 'admin', element: <AdminPage /> },
      ],
    },
  ],
  { basename: import.meta.env.BASE_URL.replace(/\/$/, '') },
)
