import type {
  DraggableAttributes,
  DraggableSyntheticListeners,
} from '@dnd-kit/core'
import {
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from '@mui/material'
import type { CSSProperties, ReactNode } from 'react'
import { useMatch, useNavigate, useResolvedPath } from 'react-router'

import { useVerge } from '@/hooks/use-verge'

interface SortableProps {
  setNodeRef?: (element: HTMLElement | null) => void
  attributes?: DraggableAttributes
  listeners?: DraggableSyntheticListeners
  style?: CSSProperties
  isDragging?: boolean
  disabled?: boolean
}

interface Props {
  to: string
  children: string
  icon: ReactNode[]
  sortable?: SortableProps
}
export const LayoutItem = (props: Props) => {
  const { to, children, icon, sortable } = props
  const { verge } = useVerge()
  const { menu_icon } = verge ?? {}
  const navCollapsed = verge?.collapse_navbar ?? false
  const resolved = useResolvedPath(to)
  const match = useMatch({ path: resolved.pathname, end: true })
  const navigate = useNavigate()

  const effectiveMenuIcon =
    navCollapsed && menu_icon === 'disable' ? 'monochrome' : menu_icon

  const { setNodeRef, attributes, listeners, style, isDragging, disabled } =
    sortable ?? {}

  const draggable = Boolean(sortable) && !disabled
  const dragHandleProps = draggable
    ? { ...(attributes ?? {}), ...(listeners ?? {}) }
    : undefined

  return (
    <ListItem
      ref={setNodeRef}
      style={style}
      sx={[
        { py: 0.5, maxWidth: 250, mx: 'auto', padding: '4px 0px' },
        isDragging ? { opacity: 0.78 } : {},
      ]}
    >
      <ListItemButton
        selected={!!match}
        {...(dragHandleProps ?? {})}
        sx={[
          {
            borderRadius: 2,
            marginLeft: 1.25,
            paddingLeft: 1,
            paddingRight: 1,
            marginRight: 1.25,
            cursor: draggable ? 'grab' : 'pointer',
            '&:active': draggable ? { cursor: 'grabbing' } : {},
            '& .MuiListItemText-primary': {
              color: 'text.primary',
              fontWeight: '700',
            },
          },
          ({ palette: { mode } }) => {
            const color = mode === 'light' ? '#003366' : '#ffffff'
            const crystalBg = mode === 'light'
              ? 'linear-gradient(to bottom, rgba(14, 144, 255, 0.22) 0%, rgba(14, 144, 255, 0.08) 45%, rgba(14, 144, 255, 0.16) 50%, rgba(14, 144, 255, 0.32) 100%)'
              : 'linear-gradient(to bottom, rgba(14, 144, 255, 0.4) 0%, rgba(14, 144, 255, 0.15) 45%, rgba(14, 144, 255, 0.25) 50%, rgba(14, 144, 255, 0.48) 100%)'
            const crystalBorder = mode === 'light'
              ? '1px solid rgba(14, 144, 255, 0.45)'
              : '1px solid rgba(14, 144, 255, 0.7)'
            const crystalShadow = mode === 'light'
              ? '0 1px 2px rgba(14, 144, 255, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.5)'
              : '0 1px 3px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.25), 0 0 6px rgba(14, 144, 255, 0.3)'
            return {
              '&.Mui-selected': { 
                background: `${crystalBg} !important`,
                border: crystalBorder,
                boxShadow: crystalShadow,
              },
              '&.Mui-selected:hover': { 
                background: `${crystalBg} !important`,
                filter: 'brightness(1.08)',
              },
              '&.Mui-selected .MuiListItemText-primary': { 
                color,
                textShadow: mode === 'light' ? '0 1px 1px rgba(255, 255, 255, 0.8)' : '0 1px 2px rgba(0, 0, 0, 0.5)',
              },
            }
          },
        ]}
        title={navCollapsed ? children : undefined}
        aria-label={navCollapsed ? children : undefined}
        onClick={() => navigate(to)}
      >
        {(effectiveMenuIcon === 'monochrome' || !effectiveMenuIcon) && (
          <ListItemIcon
            sx={{
              color: 'text.primary',
              marginLeft: '6px',
              cursor: draggable ? 'grab' : 'inherit',
            }}
          >
            {icon[0]}
          </ListItemIcon>
        )}
        {effectiveMenuIcon === 'colorful' && (
          <ListItemIcon sx={{ cursor: draggable ? 'grab' : 'inherit' }}>
            {icon[1]}
          </ListItemIcon>
        )}
        <ListItemText
          sx={{
            textAlign: 'center',
            marginLeft: effectiveMenuIcon === 'disable' ? '' : '-35px',
          }}
          primary={children}
        />
      </ListItemButton>
    </ListItem>
  )
}
