/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useEffect, useRef, useState, useCallback } from 'react'
import type { ReactNode } from 'react'
import { Navigate } from 'react-router-dom'
import { Box, CircularProgress } from '@mui/material'
import { api } from '../api'

// 设备刚重启时后端可能尚未就绪，首次状态查询失败要重试；取不到状态时 enabled
// 停在默认的 false，RequireAuth 会放行整棵业务树并发打出十几个必然 401 的请求
const INITIAL_STATUS_RETRIES = 5
const INITIAL_STATUS_RETRY_DELAY_MS = 1000

interface AuthContextType {
  enabled: boolean
  loggedIn: boolean
  loading: boolean
  /** 是否成功取到过鉴权状态。取不到时 enabled 仍是默认的 false，不能据此认定鉴权已关闭 */
  statusKnown: boolean
  /** 是否已发起跳转登录页的整页导航 */
  redirecting: boolean
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
  const [redirecting, setRedirecting] = useState(false)
  const redirectingRef = useRef(false)

  /** 拉一次鉴权状态，返回是否取到 */
  const fetchStatus = useCallback(async () => {
    try {
      const response = await api.getAuthStatus()
      if (response.status === 'ok' && response.data) {
        setEnabled(response.data.enabled)
        setLoggedIn(response.data.logged_in)
        setStatusKnown(true)
        return true
      }
      return false
    } catch (error) {
      console.warn('获取登录状态失败:', error)
      return false
    }
  }, [])

  const refreshStatus = useCallback(async () => {
    await fetchStatus()
  }, [fetchStatus])

  useEffect(() => {
    let cancelled = false

    const loadInitialStatus = async () => {
      for (let attempt = 0; attempt <= INITIAL_STATUS_RETRIES; attempt += 1) {
        if (cancelled) {
          return
        }
        if (await fetchStatus()) {
          break
        }
        if (attempt < INITIAL_STATUS_RETRIES) {
          await new Promise((resolve) => setTimeout(resolve, INITIAL_STATUS_RETRY_DELAY_MS))
        }
      }

      if (!cancelled) {
        setLoading(false)
      }
    }

    void loadInitialStatus()

    return () => {
      cancelled = true
    }
  }, [fetchStatus])

  useEffect(() => {
    const handleUnauthorized = () => {
      // 导航提交前，已在途的请求仍会陆续 401（节流窗口只有 1 秒），再调一次
      // replace 会打断正在进行的导航重新来过
      if (redirectingRef.current) {
        return
      }
      redirectingRef.current = true

      // 只置跳转标记，不动 loggedIn：改 loggedIn 会让 RequireAuth 立刻软跳到
      // 登录页，而下面的整页导航随后又把它整个换掉，观感上就是登录页自己刷新
      // 了一次。标记同时让业务页面卸载，导航期间不再有轮询请求撞 401
      setRedirecting(true)
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
      value={{ enabled, loggedIn, loading, statusKnown, redirecting, login, logout, refreshStatus }}
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
  const { enabled, loggedIn, loading, redirecting } = useAuth()

  // redirecting 期间同样停在占位：整页导航已经发起，此时再软跳一次登录页，
  // 会让登录页先渲染一遍、随即被导航结果替换
  if (loading || redirecting) {
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
