/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useEffect, useState, useCallback } from 'react'
import type { ReactNode } from 'react'
import { Navigate } from 'react-router-dom'
import { Box, CircularProgress } from '@mui/material'
import { api } from '../api'

interface AuthContextType {
  enabled: boolean
  loggedIn: boolean
  loading: boolean
  /** 是否成功取到过鉴权状态。取不到时 enabled 仍是默认的 false，不能据此认定鉴权已关闭 */
  statusKnown: boolean
  login: (username: string, password: string) => Promise<void>
  logout: () => Promise<void>
  refreshStatus: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export function useAuth() {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within AuthProvider')
  }
  return context
}

interface AuthProviderProps {
  children: ReactNode
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [enabled, setEnabled] = useState(false)
  const [loggedIn, setLoggedIn] = useState(false)
  const [loading, setLoading] = useState(true)
  const [statusKnown, setStatusKnown] = useState(false)

  const refreshStatus = useCallback(async () => {
    try {
      const response = await api.getAuthStatus()
      if (response.status === 'ok' && response.data) {
        setEnabled(response.data.enabled)
        setLoggedIn(response.data.logged_in)
        setStatusKnown(true)
      }
    } catch (error) {
      console.warn('获取登录状态失败:', error)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void refreshStatus()
  }, [refreshStatus])

  useEffect(() => {
    const handleUnauthorized = () => {
      setLoggedIn(false)
      // 整页导航而非路由跳转：会话已失效，没有内存状态需要保留，而重新加载
      // index.html 能拿到最新的 chunk 清单。软跳转要到此刻才首次拉取登录页
      // chunk，设备刚重启时这一步很容易失败，且 React.lazy 会缓存失败结果
      window.location.replace('/login')
    }

    window.addEventListener('udx710:unauthorized', handleUnauthorized)
    return () => window.removeEventListener('udx710:unauthorized', handleUnauthorized)
  }, [])

  const login = async (username: string, password: string) => {
    const response = await api.login(username, password)
    if (response.status !== 'ok') {
      throw new Error(response.message || '登录失败')
    }
    setLoggedIn(true)
    setEnabled(true)
    setStatusKnown(true)
  }

  const logout = async () => {
    try {
      await api.logout()
    } catch (error) {
      // 会话可能已在服务端失效（改密码、别处关闭鉴权），登出请求本身 401 不影响结果
      console.warn('登出请求失败:', error)
    } finally {
      setLoggedIn(false)
    }
  }

  return (
    <AuthContext.Provider
      value={{ enabled, loggedIn, loading, statusKnown, login, logout, refreshStatus }}
    >
      {children}
    </AuthContext.Provider>
  )
}

/**
 * 路由守卫：登录状态未知时只渲染占位，未登录时直接跳登录页。
 *
 * 不加守卫时未登录用户打开首页会先挂载全部业务页面、并发打出十几个请求，
 * 全部 401 后才被动跳走，对这台 CPU 紧张的设备是一波无谓负载。
 */
export function RequireAuth({ children }: { children: ReactNode }) {
  const { enabled, loggedIn, loading } = useAuth()

  if (loading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="100vh">
        <CircularProgress />
      </Box>
    )
  }

  if (enabled && !loggedIn) {
    return <Navigate to="/login" replace />
  }

  return <>{children}</>
}
