<script setup lang="ts">
defineProps<{
  label: string
  on: boolean
  active?: boolean
}>()
</script>

<template>
  <span class="pill" :class="{ on, off: !on, active }">
    <span class="pill-dot" aria-hidden="true"></span>{{ label }}
  </span>
</template>
