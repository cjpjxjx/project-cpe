/*
 * @Author: 1orz cloudorzi@gmail.com
 * @Date: 2025-12-07 12:46:40
 * @LastEditors: 1orz cloudorzi@gmail.com
 * @LastEditTime: 2025-12-13 12:45:04
 * @FilePath: /udx710-backend/frontend/src/pages/Terminal.tsx
 * @Description: 
 * 
 * Copyright (c) 2025 by 1orz, All Rights Reserved. 
 */
import { Box, Typography, IconButton, Tooltip } from '@mui/material'
import { OpenInNew as OpenInNewIcon, Fullscreen as FullscreenIcon, FullscreenExit as FullscreenExitIcon } from '@mui/icons-material'
import { useState, useRef } from 'react'

export default function Terminal() {
  const [isFullscreen, setIsFullscreen] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)

  // ttyd 运行在同一主机的 7681 端口
  const displayUrl = `${window.location.protocol}//${window.location.hostname}:7681`

  const handleOpenInNewTab = () => {
    window.open(displayUrl, '_blank')
  }

  const handleFullscreen = () => {
    if (containerRef.current) {
      if (!document.fullscreenElement) {
        void containerRef.current.requestFullscreen()
        setIsFullscreen(true)
      } else {
        void document.exitFullscreen()
        setIsFullscreen(false)
      }
    }
  }

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          mb: 2,
        }}
      >
        <Typography variant="h5" fontWeight={600}>
          Web 终端
        </Typography>
        <Box>
          <Tooltip title="Fullscreen">
            <IconButton onClick={handleFullscreen} size="small">
              {isFullscreen ? <FullscreenExitIcon /> : <FullscreenIcon />}
            </IconButton>
          </Tooltip>
          <Tooltip title="Open in new tab">
            <IconButton onClick={handleOpenInNewTab} size="small">
              <OpenInNewIcon />
            </IconButton>
          </Tooltip>
        </Box>
      </Box>

      {/* Terminal iframe container */}
      <Box
        ref={containerRef}
        sx={{
          flexGrow: 1,
          minHeight: 0,
          borderRadius: 2,
          overflow: 'hidden',
          border: 1,
          borderColor: 'divider',
          bgcolor: '#1e1e1e',
        }}
      >
        <iframe
          src={displayUrl}
          title="Web Terminal"
          style={{
            width: '100%',
            height: '100%',
            border: 'none',
          }}
          allow="clipboard-read; clipboard-write"
        />
      </Box>

      {/* Footer hint */}
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{ mt: 1, textAlign: 'center' }}
      >
        ttyd @ {displayUrl}
      </Typography>
    </Box>
  )
}

