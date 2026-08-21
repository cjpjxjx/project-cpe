/*
 * @Author: 1orz cloudorzi@gmail.com
 * @Date: 2025-11-22 10:30:41
 * @LastEditors: 1orz cloudorzi@gmail.com
 * @LastEditTime: 2025-12-13 12:43:49
 * @FilePath: /udx710-backend/frontend/src/components/ErrorSnackbar.tsx
 * @Description: 
 * 
 * Copyright (c) 2025 by 1orz, All Rights Reserved. 
 */
import { useState } from 'react'
import {
  Snackbar,
  Alert,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Box,
} from '@mui/material'
import { Close as CloseIcon, InfoOutlined } from '@mui/icons-material'

interface ErrorSnackbarProps {
  error: string | null
  onClose: () => void
}

const TRUNCATE_LEN = 80

export default function ErrorSnackbar({ error, onClose }: ErrorSnackbarProps) {
  const [dialogOpen, setDialogOpen] = useState(false)
  // 关闭 Snackbar 时 error 会先变成 null，退场动画期间 Alert 仍挂载并重新渲染；
  // 锁存最近一条非空内容，避免退场瞬间闪现成兜底的“未知错误”
  const [displayError, setDisplayError] = useState<string | null>(null)
  const [prevError, setPrevError] = useState<string | null>(null)

  if (error !== prevError) {
    setPrevError(error)
    if (error !== null) {
      setDisplayError(error)
    }
  }

  const isLong = !!displayError && displayError.length > TRUNCATE_LEN
  const displayText = isLong ? displayError.slice(0, TRUNCATE_LEN) + '...' : (displayError ?? '未知错误')

  const handleSnackbarClose = () => {
    onClose()
    setDialogOpen(false)
  }

  return (
    <>
      <Snackbar
        open={!!error}
        autoHideDuration={6000}
        onClose={handleSnackbarClose}
        anchorOrigin={{ vertical: 'top', horizontal: 'center' }}
      >
        <Alert
          severity="error"
          variant="filled"
          onClose={handleSnackbarClose}
          action={
            <>
              {isLong && (
                <IconButton
                  size="small"
                  color="inherit"
                  onClick={() => setDialogOpen(true)}
                  title="查看完整详情"
                >
                  <InfoOutlined fontSize="small" />
                </IconButton>
              )}
              <IconButton
                size="small"
                color="inherit"
                onClick={handleSnackbarClose}
              >
                <CloseIcon fontSize="small" />
              </IconButton>
            </>
          }
          sx={{ minWidth: 300 }}
        >
          {displayText}
        </Alert>
      </Snackbar>

      <Dialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle>
          <Box display="flex" alignItems="center" gap={1}>
            <InfoOutlined color="error" />
            错误详情
          </Box>
        </DialogTitle>
        <DialogContent>
          <Box
            sx={{
              bgcolor: 'action.hover',
              p: 2,
              borderRadius: 1,
              fontFamily: 'monospace',
              fontSize: '0.875rem',
              wordBreak: 'break-word',
              whiteSpace: 'pre-wrap',
              maxHeight: 300,
              overflow: 'auto',
            }}
          >
            {displayError ?? '未知错误'}
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDialogOpen(false)}>关闭</Button>
        </DialogActions>
      </Dialog>
    </>
  )
}

