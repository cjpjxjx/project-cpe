/*
 * @Author: 1orz cloudorzi@gmail.com
 * @Date: 2025-11-23 01:05:03
 * @LastEditors: 1orz cloudorzi@gmail.com
 * @LastEditTime: 2025-12-13 12:43:58
 * @FilePath: /udx710-backend/frontend/src/contexts/ThemeContext.tsx
 * @Description: 
 * 
 * Copyright (c) 2025 by 1orz, All Rights Reserved. 
 */
/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useEffect, useMemo } from 'react'
import type { ReactNode } from 'react'
import { ThemeProvider as MuiThemeProvider, createTheme } from '@mui/material/styles'
import CssBaseline from '@mui/material/CssBaseline'

type ThemeMode = 'light' | 'dark' | 'auto'
type ResolvedThemeMode = 'light' | 'dark'

interface ThemeContextType {
  mode: ThemeMode
  resolvedMode: ResolvedThemeMode
  setMode: (mode: ThemeMode) => void
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined)

export function useTheme() {
  const context = useContext(ThemeContext)
  if (!context) {
    throw new Error('useTheme must be used within ThemeProvider')
  }
  return context
}

function getSystemPrefersDark(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches
}

interface ThemeProviderProps {
  children: ReactNode
}

export function ThemeProvider({ children }: ThemeProviderProps) {
  // 从 localStorage 读取保存的主题，默认自动
  const [mode, setMode] = useState<ThemeMode>(() => {
    const saved = localStorage.getItem('theme-mode')
    return saved === 'dark' || saved === 'light' || saved === 'auto' ? saved : 'auto'
  })

  // 跟踪系统颜色模式偏好（仅在 auto 模式下需要）
  const [systemPrefersDark, setSystemPrefersDark] = useState(getSystemPrefersDark)

  // 保存主题设置到 localStorage
  useEffect(() => {
    localStorage.setItem('theme-mode', mode)
  }, [mode])

  // 监听系统颜色模式变化，供 auto 模式实时响应
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handleChange = (event: MediaQueryListEvent) => {
      setSystemPrefersDark(event.matches)
    }
    mediaQuery.addEventListener('change', handleChange)
    return () => mediaQuery.removeEventListener('change', handleChange)
  }, [])

  const resolvedMode: ResolvedThemeMode = mode === 'auto' ? (systemPrefersDark ? 'dark' : 'light') : mode

  const theme = useMemo(() => createTheme({
    palette: {
      mode: resolvedMode,
      primary: {
        main: resolvedMode === 'light' ? '#1976d2' : '#90caf9',
        light: resolvedMode === 'light' ? '#42a5f5' : '#e3f2fd',
        dark: resolvedMode === 'light' ? '#1565c0' : '#42a5f5',
      },
      secondary: {
        main: resolvedMode === 'light' ? '#dc004e' : '#f48fb1',
        light: resolvedMode === 'light' ? '#f50057' : '#f8bbd0',
        dark: resolvedMode === 'light' ? '#c51162' : '#ec407a',
      },
      success: {
        main: resolvedMode === 'light' ? '#2e7d32' : '#66bb6a',
        light: resolvedMode === 'light' ? '#4caf50' : '#81c784',
        dark: resolvedMode === 'light' ? '#1b5e20' : '#388e3c',
      },
      warning: {
        main: resolvedMode === 'light' ? '#ed6c02' : '#ffa726',
        light: resolvedMode === 'light' ? '#ff9800' : '#ffb74d',
        dark: resolvedMode === 'light' ? '#e65100' : '#f57c00',
      },
      error: {
        main: resolvedMode === 'light' ? '#d32f2f' : '#f44336',
        light: resolvedMode === 'light' ? '#ef5350' : '#e57373',
        dark: resolvedMode === 'light' ? '#c62828' : '#d32f2f',
      },
      info: {
        main: resolvedMode === 'light' ? '#0288d1' : '#29b6f6',
        light: resolvedMode === 'light' ? '#03a9f4' : '#4fc3f7',
        dark: resolvedMode === 'light' ? '#01579b' : '#0277bd',
      },
      background: {
        default: resolvedMode === 'light' ? '#f5f5f5' : '#121212',
        paper: resolvedMode === 'light' ? '#ffffff' : '#1e1e1e',
      },
    },
    typography: {
      fontFamily: [
        '-apple-system',
        'BlinkMacSystemFont',
        '"Segoe UI"',
        'Roboto',
        '"Helvetica Neue"',
        'Arial',
        'sans-serif',
        '"Apple Color Emoji"',
        '"Segoe UI Emoji"',
        '"Segoe UI Symbol"',
      ].join(','),
      h4: {
        fontWeight: 600,
      },
      h5: {
        fontWeight: 600,
      },
      h6: {
        fontWeight: 600,
      },
    },
    components: {
      MuiCssBaseline: {
        styleOverrides: {
          body: {
            scrollbarColor: resolvedMode === 'dark' ? '#6b6b6b #2b2b2b' : '#c1c1c1 #f1f1f1',
            '&::-webkit-scrollbar, & *::-webkit-scrollbar': {
              width: 8,
              height: 8,
            },
            '&::-webkit-scrollbar-thumb, & *::-webkit-scrollbar-thumb': {
              borderRadius: 4,
              backgroundColor: resolvedMode === 'dark' ? '#6b6b6b' : '#c1c1c1',
            },
            '&::-webkit-scrollbar-track, & *::-webkit-scrollbar-track': {
              backgroundColor: resolvedMode === 'dark' ? '#2b2b2b' : '#f1f1f1',
            },
          },
        },
      },
      MuiCard: {
        defaultProps: {
          elevation: resolvedMode === 'dark' ? 3 : 2,
        },
        styleOverrides: {
          root: {
            borderRadius: 12,
          },
        },
      },
      MuiButton: {
        defaultProps: {
          disableElevation: true,
        },
        styleOverrides: {
          root: {
            borderRadius: 8,
            textTransform: 'none',
            fontWeight: 500,
          },
        },
      },
      MuiPaper: {
        styleOverrides: {
          root: {
            borderRadius: 12,
          },
        },
      },
      MuiAppBar: {
        styleOverrides: {
          root: {
            borderRadius: 0,
          },
        },
      },
      MuiChip: {
        styleOverrides: {
          root: {
            borderRadius: 8,
            fontWeight: 500,
          },
        },
      },
    },
    shape: {
      borderRadius: 8,
    },
  }), [resolvedMode])

  return (
    <ThemeContext.Provider value={{ mode, resolvedMode, setMode }}>
      <MuiThemeProvider theme={theme}>
        <CssBaseline />
        {children}
      </MuiThemeProvider>
    </ThemeContext.Provider>
  )
}

