import { Box, Card, CardContent, Typography, LinearProgress } from '@mui/material'
import { Storage } from '@mui/icons-material'
import { formatBytes, getMemoryColor } from '../utils'
import type { SystemStatsResponse } from '@/api/types'

interface DiskUsageProps {
  systemStats: SystemStatsResponse | null
}

export function DiskUsage({ systemStats }: DiskUsageProps) {
  return (
    <Card>
      <CardContent>
        <Box display="flex" alignItems="center" gap={1} mb={1.5}>
          <Storage color="primary" />
          <Typography variant="subtitle2" color="text.secondary">
            磁盘使用
          </Typography>
        </Box>
        {systemStats?.disk && systemStats.disk.length > 0 ? (
          <Box>
            {systemStats.disk.map((disk, idx) => (
              <Box key={idx} sx={{ mb: idx < systemStats.disk.length - 1 ? 1 : 0 }}>
                <Box display="flex" justifyContent="space-between" alignItems="center" mb={0.5}>
                  <Typography variant="caption" color="text.secondary" noWrap sx={{ maxWidth: '55%' }}>
                    {disk.mount_point}
                  </Typography>
                  <Typography variant="caption" fontWeight="medium">
                    {formatBytes(disk.used_bytes)} / {formatBytes(disk.total_bytes)}
                  </Typography>
                </Box>
                <LinearProgress
                  variant="determinate"
                  value={disk.used_percent}
                  color={getMemoryColor(disk.used_percent)}
                  sx={{ height: 4, borderRadius: 2 }}
                />
              </Box>
            ))}
          </Box>
        ) : (
          <Typography variant="body2" color="text.secondary">
            暂无数据
          </Typography>
        )}
      </CardContent>
    </Card>
  )
}
