/*
 * @Author: 1orz cloudorzi@gmail.com
 * @Date: 2025-11-22 10:30:41
 * @LastEditors: 1orz cloudorzi@gmail.com
 * @LastEditTime: 2025-12-13 12:43:28
 * @FilePath: /udx710-backend/frontend/src/components/Layout/TopBar.tsx
 * @Description: 
 * 
 * Copyright (c) 2025 by 1orz, All Rights Reserved. 
 */
/*
 * @Author: 1orz cloudorzi@gmail.com
 * @Date: 2025-11-22 10:30:41
 * @LastEditors: 1orz cloudorzi@gmail.com
 * @LastEditTime: 2025-12-13 12:43:22
 * @FilePath: /udx710-backend/frontend/src/components/Layout/TopBar.tsx
 * @Description: 
 * 
 * Copyright (c) 2025 by 1orz, All Rights Reserved. 
 */
import { useState } from 'react'
import {
  AppBar,
  Toolbar,
  Typography,
  IconButton,
  Box,
  Menu,
  MenuItem,
  ListItemIcon,
  ListItemText,
  Divider,
  Snackbar,
  Alert,
} from '@mui/material'
import {
  Menu as MenuIcon,
  Refresh as RefreshIcon,
  MoreVert as MoreVertIcon,
  Brightness4 as DarkModeIcon,
  Brightness7 as LightModeIcon,
  BrightnessAuto as AutoModeIcon,
  Speed as SpeedIcon,
  Palette as PaletteIcon,
  Logout as LogoutIcon,
  RestartAlt as RestartAltIcon,
} from '@mui/icons-material'
import { api } from '../../api'
import ConfirmDialog from '../ConfirmDialog'
import ErrorSnackbar from '../ErrorSnackbar'
import { useTheme } from '../../contexts/ThemeContext'
import { useRefreshInterval } from '../../contexts/RefreshContext'
import { useAuth } from '../../contexts/AuthContext'

const REBOOT_DELAY_SECONDS = 3

interface TopBarProps {
  drawerWidth: number
  onMenuClick: () => void
  refreshInterval: number
  onRefreshIntervalChange: (interval: number) => void
}

export default function TopBar({
  drawerWidth,
  onMenuClick,
  refreshInterval,
  onRefreshIntervalChange,
}: TopBarProps) {
  const { mode, setMode } = useTheme()
  const { triggerRefresh } = useRefreshInterval()
  const { enabled: authEnabled, logout } = useAuth()
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null)
  const [refreshMenuAnchor, setRefreshMenuAnchor] = useState<null | HTMLElement>(null)
  const [themeMenuAnchor, setThemeMenuAnchor] = useState<null | HTMLElement>(null)
  const [rebootConfirmOpen, setRebootConfirmOpen] = useState(false)
  const [rebooting, setRebooting] = useState(false)
  const [rebootError, setRebootError] = useState<string | null>(null)
  const [rebootSuccess, setRebootSuccess] = useState<string | null>(null)

  const handleMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget)
  }

  const handleMenuClose = () => {
    setAnchorEl(null)
  }

  const handleRefreshMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    setRefreshMenuAnchor(event.currentTarget)
  }

  const handleRefreshMenuClose = () => {
    setRefreshMenuAnchor(null)
  }

  const handleRefreshIntervalChange = (interval: number) => {
    onRefreshIntervalChange(interval)
    handleRefreshMenuClose()
  }

  const handleThemeMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    setThemeMenuAnchor(event.currentTarget)
  }

  const handleThemeMenuClose = () => {
    setThemeMenuAnchor(null)
  }

  const handleThemeModeChange = (nextMode: 'light' | 'dark' | 'auto') => {
    setMode(nextMode)
    handleThemeMenuClose()
  }

  const handleRefresh = () => {
    triggerRefresh()
  }

  const handleLogout = () => {
    handleMenuClose()
    // 跳转交给 RequireAuth：logout() 置 loggedIn=false 后它会 replace 到登录页
    void logout()
  }

  const handleRebootClick = () => {
    handleMenuClose()
    setRebootConfirmOpen(true)
  }

  const handleRebootConfirm = () => {
    void rebootDevice()
  }

  const rebootDevice = async () => {
    setRebooting(true)
    setRebootError(null)
    try {
      await api.systemReboot(REBOOT_DELAY_SECONDS)
      setRebootConfirmOpen(false)
      setRebootSuccess(`设备将在 ${REBOOT_DELAY_SECONDS} 秒后重启...`)
    } catch (err) {
      setRebootError(err instanceof Error ? err.message : String(err))
    } finally {
      setRebooting(false)
    }
  }

  const getRefreshLabel = () => {
    if (refreshInterval === 0) return '手动'
    return `${refreshInterval / 1000} 秒`
  }

  const getThemeLabel = () => {
    if (mode === 'auto') return '自动'
    return mode === 'dark' ? '暗色' : '亮色'
  }

  return (
    <AppBar
      position="fixed"
      sx={{
        width: { sm: `calc(100% - ${drawerWidth}px)` },
        ml: { sm: `${drawerWidth}px` },
      }}
    >
      <Toolbar sx={{ minHeight: { xs: 56, sm: 64 } }}>
        {/* 菜单折叠按钮 - 所有屏幕尺寸都可见 */}
        <IconButton
          color="inherit"
          aria-label="切换侧边栏"
          edge="start"
          onClick={onMenuClick}
          sx={{ mr: 2 }}
        >
          <MenuIcon />
        </IconButton>

        {/* 标题 */}
        <Typography
          variant="h6"
          noWrap
          component="div"
          sx={{
            flexGrow: 1,
            fontSize: { xs: '1rem', sm: '1.25rem' },
          }}
        >
          控制面板
        </Typography>

        {/* 右侧按钮组 */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: { xs: 0.5, sm: 1 } }}>
          {/* 刷新按钮 - 始终显示 */}
          <IconButton
            color="inherit"
            onClick={handleRefresh}
            title="刷新页面"
            sx={{ display: { xs: 'inline-flex', sm: 'inline-flex' } }}
          >
            <RefreshIcon />
          </IconButton>

          {/* 更多选项按钮 - 折叠其他功能 */}
          <IconButton
            color="inherit"
            onClick={handleMenuOpen}
            title="更多选项"
            sx={{ display: { xs: 'inline-flex', sm: 'inline-flex' } }}
          >
            <MoreVertIcon />
          </IconButton>
        </Box>

        {/* 更多选项菜单 */}
        <Menu
          anchorEl={anchorEl}
          open={Boolean(anchorEl)}
          onClose={handleMenuClose}
          anchorOrigin={{
            vertical: 'bottom',
            horizontal: 'right',
          }}
          transformOrigin={{
            vertical: 'top',
            horizontal: 'right',
          }}
          PaperProps={{
            sx: {
              minWidth: 200,
              mt: 1,
            },
          }}
        >
          {/* 主题切换 */}
          <MenuItem onClick={handleThemeMenuOpen}>
            <ListItemIcon>
              <PaletteIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>颜色模式</ListItemText>
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ ml: 2, flexGrow: 1, textAlign: 'right' }}
            >
              {getThemeLabel()}
            </Typography>
          </MenuItem>

          <Divider />

          {/* 刷新频率 */}
          <MenuItem onClick={handleRefreshMenuOpen}>
            <ListItemIcon>
              <SpeedIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>刷新频率</ListItemText>
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ ml: 2, flexGrow: 1, textAlign: 'right' }}
            >
              {getRefreshLabel()}
            </Typography>
          </MenuItem>

          <Divider />

          {/* 重启设备 */}
          <MenuItem onClick={handleRebootClick}>
            <ListItemIcon>
              <RestartAltIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>重启设备</ListItemText>
          </MenuItem>

          {authEnabled && (
            <>
              <Divider />
              <MenuItem onClick={handleLogout}>
                <ListItemIcon>
                  <LogoutIcon fontSize="small" />
                </ListItemIcon>
                <ListItemText>退出登录</ListItemText>
              </MenuItem>
            </>
          )}
        </Menu>

        {/* 颜色模式子菜单 */}
        <Menu
          anchorEl={themeMenuAnchor}
          open={Boolean(themeMenuAnchor)}
          onClose={handleThemeMenuClose}
          anchorOrigin={{
            vertical: 'top',
            horizontal: 'left',
          }}
          transformOrigin={{
            vertical: 'top',
            horizontal: 'right',
          }}
          PaperProps={{
            sx: {
              minWidth: 150,
            },
          }}
        >
          <MenuItem selected={mode === 'auto'} onClick={() => handleThemeModeChange('auto')}>
            <ListItemIcon>
              <AutoModeIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>自动</ListItemText>
          </MenuItem>
          <MenuItem selected={mode === 'light'} onClick={() => handleThemeModeChange('light')}>
            <ListItemIcon>
              <LightModeIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>亮色</ListItemText>
          </MenuItem>
          <MenuItem selected={mode === 'dark'} onClick={() => handleThemeModeChange('dark')}>
            <ListItemIcon>
              <DarkModeIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>暗色</ListItemText>
          </MenuItem>
        </Menu>

        {/* 刷新频率子菜单 */}
        <Menu
          anchorEl={refreshMenuAnchor}
          open={Boolean(refreshMenuAnchor)}
          onClose={handleRefreshMenuClose}
          anchorOrigin={{
            vertical: 'top',
            horizontal: 'left',
          }}
          transformOrigin={{
            vertical: 'top',
            horizontal: 'right',
          }}
          PaperProps={{
            sx: {
              minWidth: 150,
            },
          }}
        >
          <MenuItem
            selected={refreshInterval === 5000}
            onClick={() => handleRefreshIntervalChange(5000)}
          >
            5 秒/次
          </MenuItem>
          <MenuItem
            selected={refreshInterval === 10000}
            onClick={() => handleRefreshIntervalChange(10000)}
          >
            10 秒/次
          </MenuItem>
          <MenuItem
            selected={refreshInterval === 30000}
            onClick={() => handleRefreshIntervalChange(30000)}
          >
            30 秒/次
          </MenuItem>
          <MenuItem
            selected={refreshInterval === 60000}
            onClick={() => handleRefreshIntervalChange(60000)}
          >
            60 秒/次
          </MenuItem>
          <Divider />
          <MenuItem
            selected={refreshInterval === 0}
            onClick={() => handleRefreshIntervalChange(0)}
          >
            手动刷新
          </MenuItem>
        </Menu>

        <ConfirmDialog
          open={rebootConfirmOpen}
          title="确认重启设备"
          content="重启期间设备将断开连接，确定继续？"
          confirmText="确认重启"
          confirmColor="error"
          loading={rebooting}
          onConfirm={handleRebootConfirm}
          onCancel={() => setRebootConfirmOpen(false)}
        />

        <ErrorSnackbar error={rebootError} onClose={() => setRebootError(null)} />
        <Snackbar
          open={!!rebootSuccess}
          autoHideDuration={5000}
          onClose={() => setRebootSuccess(null)}
          anchorOrigin={{ vertical: 'top', horizontal: 'center' }}
        >
          <Alert severity="success" variant="filled" onClose={() => setRebootSuccess(null)}>
            {rebootSuccess}
          </Alert>
        </Snackbar>
      </Toolbar>
    </AppBar>
  )
}
