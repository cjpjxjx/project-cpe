import { useState, type FormEvent } from 'react'
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
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        bgcolor: 'background.default',
        p: 2,
      }}
    >
      <Card sx={{ maxWidth: 380, width: '100%' }}>
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
