/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useEffect, useState, useCallback } from 'react'
import type { ReactNode } from 'react'
import { useNavigate } from 'react-router-dom'
import { api } from '../api'

interface AuthContextType {
  enabled: boolean
  loggedIn: boolean
  loading: boolean
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
  const navigate = useNavigate()

  const refreshStatus = useCallback(async () => {
    try {
      const response = await api.getAuthStatus()
      if (response.status === 'ok' && response.data) {
        setEnabled(response.data.enabled)
        setLoggedIn(response.data.logged_in)
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
      void navigate('/login')
    }

    window.addEventListener('udx710:unauthorized', handleUnauthorized)
    return () => window.removeEventListener('udx710:unauthorized', handleUnauthorized)
  }, [navigate])

  const login = async (username: string, password: string) => {
    const response = await api.login(username, password)
    if (response.status !== 'ok') {
      throw new Error(response.message || '登录失败')
    }
    setLoggedIn(true)
    setEnabled(true)
  }

  const logout = async () => {
    try {
      await api.logout()
    } finally {
      setLoggedIn(false)
    }
  }

  return (
    <AuthContext.Provider value={{ enabled, loggedIn, loading, login, logout, refreshStatus }}>
      {children}
    </AuthContext.Provider>
  )
}
