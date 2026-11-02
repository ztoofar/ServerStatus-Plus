<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  value: number
  text?: string
  active?: boolean
}>()

const pct = computed(() => {
  if (props.active === false) return 0
  return Math.max(0, Math.min(100, Math.round(Number.isFinite(props.value) ? props.value : 0)))
})
const tone = computed(() => (pct.value >= 90 ? 'danger' : pct.value >= 75 ? 'warn' : 'ok'))
</script>

<template>
  <div class="meter" :class="{ inactive: active === false }">
    <div class="meter-track">
      <div class="meter-fill" :class="tone" :style="{ width: `${pct}%` }"></div>
    </div>
    <span class="meter-text">{{ text ?? `${pct}%` }}</span>
  </div>
</template>
