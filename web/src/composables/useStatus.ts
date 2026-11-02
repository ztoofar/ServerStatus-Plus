import { computed, onMounted, onUnmounted, ref } from 'vue'
import { fetchStatus, subscribeStatus } from '../api'
import type { StatusResponse } from '../types'

export function useStatus() {
  const status = ref<StatusResponse | null>(null)
  const live = ref(false)
  const error = ref('')
  const now = ref(Date.now() / 1000)
  let unsubscribe: (() => void) | null = null
  let pollTimer: number | null = null
  let clockTimer: number | null = null

  const age = computed(() => {
    if (!status.value) return 0
    return Math.max(0, Math.floor(now.value - status.value.updated))
  })

  async function pollOnce() {
    try {
      status.value = await fetchStatus()
      error.value = ''
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    }
  }

  function startPolling() {
    if (pollTimer !== null) return
    void pollOnce()
    pollTimer = window.setInterval(() => {
      void pollOnce()
    }, 3000)
  }

  function stopPolling() {
    if (pollTimer !== null) {
      window.clearInterval(pollTimer)
      pollTimer = null
    }
  }

  function connect() {
    unsubscribe?.()
    unsubscribe = subscribeStatus(
      (data) => {
        status.value = data
        error.value = ''
      },
      (isLive) => {
        live.value = isLive
        if (isLive) stopPolling()
        else startPolling()
      },
    )
  }

  onMounted(() => {
    void pollOnce()
    connect()
    clockTimer = window.setInterval(() => {
      now.value = Date.now() / 1000
    }, 1000)
  })

  onUnmounted(() => {
    unsubscribe?.()
    stopPolling()
    if (clockTimer !== null) window.clearInterval(clockTimer)
  })

  return { status, live, error, age }
}
