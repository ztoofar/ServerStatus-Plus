const UNITS = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB']

export function formatBytes(value: number, digits = 1): string {
  if (!Number.isFinite(value) || value <= 0) return '0 B'
  let n = value
  let unit = 0
  while (n >= 1024 && unit < UNITS.length - 1) {
    n /= 1024
    unit++
  }
  return `${n.toFixed(unit === 0 ? 0 : digits)} ${UNITS[unit]}`
}

export function formatRate(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return '0B'
  if (value < 1000) return `${value.toFixed(0)}B`
  if (value < 1000 * 1000) return `${(value / 1000).toFixed(0)}K`
  return `${(value / 1000 / 1000).toFixed(1)}M`
}

export function formatTraffic(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return '0B'
  if (value < 1024) return `${value.toFixed(0)}B`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(0)}K`
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)}M`
  if (value < 1024 * 1024 * 1024 * 1024)
    return `${(value / 1024 / 1024 / 1024).toFixed(2)}G`
  return `${(value / 1024 / 1024 / 1024 / 1024).toFixed(2)}T`
}

export function formatUptime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '—'
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  if (days > 0) return `${days} 天 ${hours} 小时`
  if (hours > 0) return `${hours} 小时 ${minutes} 分`
  return `${minutes} 分钟`
}

export function percent(used: number, total: number): number {
  if (!total || total <= 0) return 0
  return Math.max(0, Math.min(100, Math.round((used / total) * 100)))
}

export function formatAge(unix: number): string {
  if (!unix) return '从未上报'
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - unix)
  if (seconds < 5) return '刚刚'
  if (seconds < 60) return `${seconds} 秒前`
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟前`
  if (seconds < 86400) return `${Math.floor(seconds / 3600)} 小时前`
  return `${Math.floor(seconds / 86400)} 天前`
}
