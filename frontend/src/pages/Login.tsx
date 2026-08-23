import { useEffect, useState, type FormEvent } from 'react'
import { Navigate, useNavigate } from 'react-router-dom'
import {
  Box,
  Card,
  CardContent,
  TextField,
  Button,
  Typography,
  Alert,
  CircularProgress,
} from '@mui/material'
import { useAuth } from '../contexts/AuthContext'

export default function Login() {
  const { login, enabled, loggedIn, loading: authLoading, statusKnown } = useAuth()
  const navigate = useNavigate()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  // 移动端键盘弹出只缩小视觉视口，布局视口不变，100vh 的容器仍按整屏居中，
  // 卡片会落进被键盘遮住的下半屏。容器改为跟随视觉视口定位与收缩，键盘一动就居中到可见区
  const [viewport, setViewport] = useState<{ height: number; offsetTop: number } | null>(null)

  useEffect(() => {
    const vv = window.visualViewport
    if (!vv) return

    const sync = () => setViewport({ height: vv.height, offsetTop: vv.offsetTop })
    sync()
    vv.addEventListener('resize', sync)
    vv.addEventListener('scroll', sync)
    return () => {
      vv.removeEventListener('resize', sync)
      vv.removeEventListener('scroll', sync)
    }
  }, [])

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault()
    setLoading(true)
    setError(null)
    try {
      await login(username, password)
      void navigate('/', { replace: true })
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  if (authLoading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="100vh">
        <CircularProgress />
      </Box>
    )
  }

  // 鉴权已关闭时后端无条件拒绝登录，停在此页会陷入「密码怎么填都不对」的死角。
  // 必须限定 statusKnown：状态没取到时 enabled 也是 false，据此跳首页会与
  // RequireAuth 来回弹跳
  if (statusKnown && (!enabled || loggedIn)) {
    return <Navigate to="/" replace />
  }

  return (
    <Box
      sx={{
        position: 'fixed',
        left: 0,
        right: 0,
        top: viewport ? `${viewport.offsetTop}px` : 0,
        height: viewport ? `${viewport.height}px` : '100vh',
        display: 'flex',
        overflowY: 'auto',
        bgcolor: 'background.default',
        p: 2,
      }}
    >
      {/* m: auto 而非 alignItems: center：卡片高于可见区时不裁掉顶部 */}
      <Card sx={{ maxWidth: 380, width: '100%', m: 'auto' }}>
        <CardContent sx={{ p: 4 }}>
          <Box display="flex" flexDirection="column" alignItems="center" mb={3}>
            <Typography
              variant="h4"
              fontWeight={600}
              sx={{ fontSize: { xs: '1.5rem', sm: '2.125rem' }, whiteSpace: 'nowrap' }}
            >
              UDX710 控制面板
            </Typography>
          </Box>

          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
            </Alert>
          )}

          <Box component="form" onSubmit={(e: FormEvent) => void handleSubmit(e)}>
            <TextField
              label="用户名"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              fullWidth
              autoFocus
              margin="normal"
              autoComplete="username"
            />
            <TextField
              label="密码"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              fullWidth
              margin="normal"
              autoComplete="current-password"
            />
            <Button
              type="submit"
              variant="contained"
              fullWidth
              size="large"
              sx={{ mt: 3 }}
              disabled={loading}
              startIcon={loading ? <CircularProgress size={20} /> : undefined}
            >
              {loading ? '登录中...' : '登录'}
            </Button>
          </Box>
        </CardContent>
      </Card>
    </Box>
  )
}
