import HomeRoundedIcon from '@mui/icons-material/HomeRounded'
import LockOpenRoundedIcon from '@mui/icons-material/LockOpenRounded'
import { createBrowserRouter, RouteObject } from 'react-router'

import Layout from './_layout'
import UnlockPage from './unlock'

export const navItems = [
  {
    label: 'layout.components.navigation.tabs.home',
    path: '/',
    icon: [<HomeRoundedIcon key="mui" />],
    Component: () => null, // Main content is rendered directly by Layout
  },
  {
    label: 'layout.components.navigation.tabs.unlock',
    path: '/unlock',
    icon: [<LockOpenRoundedIcon key="mui" />],
    Component: UnlockPage,
  },
]

export const router = createBrowserRouter([
  {
    path: '/',
    Component: Layout,
    children: navItems.map(
      (item) =>
        ({
          path: item.path,
          Component: item.Component,
        }) as RouteObject,
    ),
  },
])
