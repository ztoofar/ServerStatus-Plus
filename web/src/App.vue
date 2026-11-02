<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useStatus } from './composables/useStatus'
import { formatBytes, formatRate, formatUptime, percent } from './format'
import MeterBar from './components/MeterBar.vue'
import StatusPill from './components/StatusPill.vue'
import type { ServerStatus } from './types'

const { status, error } = useStatus()
const theme = ref<'dark' | 'light'>('dark')
const expandedId = ref<string | null>(null)

const servers = computed(() => status.value?.servers ?? [])
const updatedText = computed(() => {
  if (!status.value) return '正在加载…'
  const time = new Date(status.value.updated * 1000)
  return `最近更新 ${time.toLocaleTimeString('zh-CN', { hour12: false })}`
})

watch(
  theme,
  (value) => {
    document.documentElement.dataset.theme = value
  },
  { immediate: true },
)

onMounted(() => {
  theme.value = window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
})

function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
}

function toggle(server: ServerStatus) {
  if (!server.connected) return
  expandedId.value = expandedId.value === server.id ? null : server.id
}

function cpu(server: ServerStatus) {
  return Math.max(0, Math.min(100, Number.isFinite(server.cpu) ? server.cpu : 0))
}

function memory(server: ServerStatus) {
  return percent(server.memory_used, server.memory_total)
}

function disk(server: ServerStatus) {
  return percent(server.hdd_used, server.hdd_total)
}

function load(server: ServerStatus) {
  return Number.isFinite(server.load) ? server.load.toFixed(2) : '–'
}

function network(server: ServerStatus) {
  return server.connected
    ? `${formatRate(server.network_rx)} | ${formatRate(server.network_tx)}`
    : '–'
}

function traffic(server: ServerStatus) {
  return server.connected
    ? `${formatBytes(server.network_in)} | ${formatBytes(server.network_out)}`
    : '–'
}

function uptime(server: ServerStatus) {
  return server.connected ? formatUptime(server.uptime) : '–'
}
</script>

<template>
  <div class="app">
    <div v-if="error && !status" class="notice error">无法连接服务端：{{ error }}</div>
    <div v-else-if="!status" class="notice">正在加载…</div>
    <div v-else class="table-wrap">
      <table class="server-table">
        <thead>
          <tr>
            <th>状态</th>
            <th>节点名</th>
            <th>虚拟化</th>
            <th>位置</th>
            <th>在线时间</th>
            <th>负载</th>
            <th>网络(B/s) ↓|↑</th>
            <th>流量(B) ↓|↑</th>
            <th>CPU</th>
            <th>内存</th>
            <th>硬盘</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="server in servers" :key="server.id">
            <tr
              class="server-row"
              :class="{ offline: !server.connected, expanded: expandedId === server.id }"
              tabindex="0"
              @click="toggle(server)"
              @keydown.enter="toggle(server)"
              @keydown.space.prevent="toggle(server)"
            >
              <td class="status-cell">
                <template v-if="server.connected">
                  <StatusPill label="IPv4" :on="server.online4" />
                  <StatusPill label="IPv6" :on="server.online6" />
                </template>
                <span v-else class="offline-label">离线</span>
              </td>
              <td class="name-cell">{{ server.name }}</td>
              <td>{{ server.type || '–' }}</td>
              <td>{{ server.location || '–' }}</td>
              <td>{{ uptime(server) }}</td>
              <td>{{ server.connected ? load(server) : '–' }}</td>
              <td>{{ network(server) }}</td>
              <td>{{ traffic(server) }}</td>
              <td class="meter-cell">
                <MeterBar
                  :value="cpu(server)"
                  :text="server.connected ? `${Math.round(cpu(server))}%` : '–'"
                  :active="server.connected"
                />
              </td>
              <td class="meter-cell">
                <MeterBar
                  :value="memory(server)"
                  :text="server.connected ? `${memory(server)}%` : '–'"
                  :active="server.connected"
                />
              </td>
              <td class="meter-cell">
                <MeterBar
                  :value="disk(server)"
                  :text="server.connected ? `${disk(server)}%` : '–'"
                  :active="server.connected"
                />
              </td>
            </tr>
            <tr v-if="expandedId === server.id && server.connected" class="expand-row">
              <td colspan="11">
                <div class="expand-grid">
                  <div>
                    <span>内存信息</span>
                    <strong>
                      {{ formatBytes(server.memory_used * 1024) }} /
                      {{ formatBytes(server.memory_total * 1024) }}
                    </strong>
                  </div>
                  <div>
                    <span>交换分区</span>
                    <strong>
                      {{ formatBytes(server.swap_used * 1024) }} /
                      {{ formatBytes(server.swap_total * 1024) }}
                    </strong>
                  </div>
                  <div>
                    <span>硬盘信息</span>
                    <strong>
                      {{ formatBytes(server.hdd_used * 1024 * 1024) }} /
                      {{ formatBytes(server.hdd_total * 1024 * 1024) }}
                    </strong>
                  </div>
                  <div v-if="server.custom">
                    <span>自定义</span>
                    <strong>{{ server.custom }}</strong>
                  </div>
                </div>
              </td>
            </tr>
          </template>
          <tr v-if="!servers.length">
            <td colspan="11" class="empty">还没有配置节点。</td>
          </tr>
        </tbody>
      </table>
    </div>

    <footer class="statusbar">
      <span class="updated">{{ updatedText }}</span>
      <button
        class="theme-btn"
        type="button"
        :aria-label="theme === 'dark' ? '切换到浅色主题' : '切换到深色主题'"
        @click="toggleTheme"
      >
        {{ theme === 'dark' ? '☀' : '☾' }}
      </button>
    </footer>
  </div>
</template>
