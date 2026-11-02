export interface Site {
  title: string
  announcement: string
}

export interface ServerStatus {
  id: string
  name: string
  type: string
  host: string
  location: string
  group?: string
  tags?: string[]
  connected: boolean
  connected_via?: 'v4' | 'v6' | ''
  last_seen: number
  uptime: number
  load: number
  network_rx: number
  network_tx: number
  network_in: number
  network_out: number
  memory_total: number
  memory_used: number
  swap_total: number
  swap_used: number
  hdd_total: number
  hdd_used: number
  cpu: number
  online4: boolean
  online6: boolean
  custom?: string
}

export interface StatusResponse {
  site: Site
  updated: number
  servers: ServerStatus[]
}
