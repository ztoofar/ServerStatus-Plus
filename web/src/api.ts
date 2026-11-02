import type { StatusResponse } from './types'

export async function fetchStatus(): Promise<StatusResponse> {
  const res = await fetch('/api/status', { cache: 'no-store' })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  return (await res.json()) as StatusResponse
}

export function subscribeStatus(
  onData: (status: StatusResponse) => void,
  onLiveChange: (live: boolean) => void,
): () => void {
  const events = new EventSource('/api/events')
  events.onopen = () => onLiveChange(true)
  events.onerror = () => onLiveChange(false)
  events.onmessage = (event) => {
    try {
      onData(JSON.parse(event.data) as StatusResponse)
    } catch {
      // Ignore malformed frames; the next event will replace the state.
    }
  }
  return () => events.close()
}
