import { Component, type ErrorInfo, type ReactNode } from 'react'
import { Box, Button, Typography } from '@mui/material'

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
}

/**
 * 渲染期异常兜底，避免整棵树被卸载后只剩白屏。
 *
 * chunk 加载失败由 lazyWithReload 处理，这里兜的是其余渲染期异常。
 */
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false }

  static getDerivedStateFromError(): State {
    return { hasError: true }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('页面渲染失败:', error, info.componentStack)
  }

  render() {
    if (!this.state.hasError) {
      return this.props.children
    }

    return (
      <Box
        display="flex"
        flexDirection="column"
        alignItems="center"
        justifyContent="center"
        gap={2}
        minHeight="100vh"
        p={2}
      >
        <Typography variant="h6">页面加载失败</Typography>
        <Typography variant="body2" color="text.secondary">
          设备可能正在启动中，请稍后重试
        </Typography>
        <Button variant="contained" onClick={() => window.location.reload()}>
          重新加载
        </Button>
      </Box>
    )
  }
}
