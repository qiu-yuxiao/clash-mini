import HomeRoundedIcon from '@mui/icons-material/HomeRounded'
import { createBrowserRouter, RouteObject } from 'react-router'

import Layout from './_layout'

export const navItems = [
  {
    label: 'layout.components.navigation.tabs.home',
    path: '/',
    icon: [<HomeRoundedIcon key="mui" />],
    Component: () => null, // Main content is rendered directly by Layout
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
