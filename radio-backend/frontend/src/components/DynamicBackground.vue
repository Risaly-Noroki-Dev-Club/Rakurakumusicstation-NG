<script setup lang="ts">
import { watch, ref } from 'vue'
import { store } from '../store'

const props = defineProps<{
  coverSrc: string
}>()

const canvasRef = ref<HTMLCanvasElement | null>(null)

function extractColor(src: string) {
  const img = new Image()
  img.crossOrigin = 'anonymous'
  img.onload = () => {
    const canvas = document.createElement('canvas')
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const size = 64
    canvas.width = size
    canvas.height = size
    ctx.drawImage(img, 0, 0, size, size)

    const data = ctx.getImageData(0, 0, size, size).data
    let r = 0, g = 0, b = 0, count = 0

    // Sample every 4th pixel for performance
    for (let i = 0; i < data.length; i += 16) {
      const pr = data[i]
      const pg = data[i + 1]
      const pb = data[i + 2]
      const pa = data[i + 3]
      if (pa < 128) continue
      // Skip near-white and near-black
      if (pr > 240 && pg > 240 && pb > 240) continue
      if (pr < 15 && pg < 15 && pb < 15) continue
      r += pr
      g += pg
      b += pb
      count++
    }

    if (count > 0) {
      r = Math.round(r / count)
      g = Math.round(g / count)
      b = Math.round(b / count)
      store.extractedColor = `rgb(${r}, ${g}, ${b})`
    }
  }
  img.onerror = () => {
    store.extractedColor = '#003D99'
  }
  img.src = src
}

watch(() => props.coverSrc, (src) => {
  if (src) {
    extractColor(src)
  } else {
    store.extractedColor = '#003D99'
  }
}, { immediate: true })
</script>

<template>
  <div
    class="am-dynamic-bg"
    :style="{ backgroundColor: store.extractedColor }"
  />
</template>

<style scoped>
.am-dynamic-bg {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 0;
  opacity: 0.08;
  transition: background-color 0.8s ease;
  pointer-events: none;
}
</style>
