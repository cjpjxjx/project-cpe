/*
 * @Author: 1orz cloudorzi@gmail.com
 * @Date: 2025-12-10 09:19:05
 * @LastEditors: 1orz cloudorzi@gmail.com
 * @LastEditTime: 2025-12-13 12:45:12
 * @FilePath: /udx710-backend/frontend/src/App.tsx
 * @Description: 
 * 
 * Copyright (c) 2025 by 1orz, All Rights Reserved. 
 */
import { createElement, lazy, Suspense, useEffect, type ComponentType, type LazyExoticComponent } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom'
import { QueryClientProvider } from '@tanstack/react-query'
import { Box, CircularProgress } from '@mui/material'
import type { SvgIconProps } from '@mui/material'
import {
  Dashboard as DashboardIcon,
  Devices as DevicesIcon,
  SignalCellularAlt as SignalIcon,
  Settings as SettingsIcon,
  Terminal as TerminalIcon,
  Phone as PhoneIcon,
  Sms as SmsIcon,
  WebAsset as WebTerminalIcon,
  SystemUpdateAlt as OtaIcon,
  RocketLaunch as InitScriptIcon,
  Login as LoginIcon,
} from '@mui/icons-material'
import { ThemeProvider } from './contexts/ThemeContext'
import { AuthProvider, RequireAuth } from './contexts/AuthContext'
import { queryClient } from './lib/queryClient'
import MainLayout from './components/Layout/MainLayout'

// 路由级别代码分割 - 按需加载页面组件
const Dashboard = lazy(() => import('./pages/Dashboard'))
const DeviceInfo = lazy(() => import('./pages/DeviceInfo'))
const Network = lazy(() => import('./pages/Network'))
const Phone = lazy(() => import('./pages/Phone'))
const SMS = lazy(() => import('./pages/SMS'))
const Configuration = lazy(() => import('./pages/Configuration'))
const InitScript = lazy(() => import('./pages/InitScript'))
const ATConsole = lazy(() => import('./pages/ATConsole'))
const Terminal = lazy(() => import('./pages/Terminal'))
const OtaUpdate = lazy(() => import('./pages/OtaUpdate'))
const Login = lazy(() => import('./pages/Login'))

// 页面加载中的 fallback
function PageLoading() {
  return (
    <Box display="flex" justifyContent="center" alignItems="center" minHeight="50vh">
      <CircularProgress size={32} />
    </Box>
  )
}

type LazyPageComponent = LazyExoticComponent<ComponentType>

interface AppRouteConfig {
  path?: string
  index?: boolean
  component: LazyPageComponent
  title: string
  icon: ComponentType<SvgIconProps>
}

const appRoutes: AppRouteConfig[] = [
  { index: true, component: Dashboard, title: '仪表盘', icon: DashboardIcon },
  { path: 'device', component: DeviceInfo, title: '设备信息', icon: DevicesIcon },
  { path: 'network', component: Network, title: '网络状态', icon: SignalIcon },
  { path: 'phone', component: Phone, title: '电话管理', icon: PhoneIcon },
  { path: 'sms', component: SMS, title: '短信管理', icon: SmsIcon },
  { path: 'config', component: Configuration, title: '系统配置', icon: SettingsIcon },
  { path: 'init-script', component: InitScript, title: '开机脚本', icon: InitScriptIcon },
  { path: 'ota', component: OtaUpdate, title: 'OTA 更新', icon: OtaIcon },
  { path: 'at-console', component: ATConsole, title: 'AT 控制台', icon: TerminalIcon },
  { path: 'terminal', component: Terminal, title: 'Web 终端', icon: WebTerminalIcon },
]

/** 用 MUI 图标生成 favicon data URL */
function generateFavicon(IconComponent: ComponentType<SvgIconProps>): string {
  const svgMarkup = renderToStaticMarkup(createElement(IconComponent))

  // MUI SvgIcon 输出的 <svg> 需要注入 xmlns 才能作为独立 SVG 使用
  const svgWithNs = svgMarkup.replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" ')

  return `data:image/svg+xml,${encodeURIComponent(svgWithNs)}`
}

/** 根据当前路由动态更新浏览器标题和 favicon */
function DocumentTitleAndFavicon() {
  const location = useLocation()

  useEffect(() => {
    const link = document.querySelector<HTMLLinkElement>('link[rel="icon"]')

    if (location.pathname === '/login') {
      document.title = 'UDX710 - 登录'
      if (link) link.href = generateFavicon(LoginIcon)
      return
    }

    const route = appRoutes.find((r) =>
      r.index ? location.pathname === '/' : location.pathname === `/${r.path}`
    )

    // 更新标题
    document.title = route ? `UDX710 - ${route.title}` : 'UDX710'

    // 更新 favicon
    if (link && route) {
      link.href = generateFavicon(route.icon)
    }
  }, [location.pathname])

  return null
}

function renderLazyPage(PageComponent: LazyPageComponent) {
  return (
    <Suspense fallback={<PageLoading />}>
      <PageComponent />
    </Suspense>
  )
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <BrowserRouter>
          <AuthProvider>
            <DocumentTitleAndFavicon />
            <Routes>
              <Route path="/login" element={renderLazyPage(Login)} />
              <Route
                path="/"
                element={
                  <RequireAuth>
                    <MainLayout />
                  </RequireAuth>
                }
              >
                {appRoutes.map((route) => (
                  <Route
                    key={route.path ?? 'index'}
                    index={route.index}
                    path={route.path}
                    element={renderLazyPage(route.component)}
                  />
                ))}
                {/* 旧路由重定向到网络状态页面 */}
                <Route path="network-interfaces" element={<Navigate to="/network" replace />} />
                <Route path="band-lock" element={<Navigate to="/network" replace />} />
              </Route>
            </Routes>
          </AuthProvider>
        </BrowserRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

export default App
