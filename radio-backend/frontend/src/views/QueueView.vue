<script setup lang="ts">
import { onMounted } from 'vue'
import { store } from '../store'
import { refreshQueue, removeQueueItem } from '../api'

onMounted(() => { refreshQueue() })
</script>

<template>
  <div>
    <div class="card">
      <h2>📋 公共队列 <span style="font-size:0.75em;color:var(--text-muted)">({{ store.queue.length }}首)</span></h2>
      <div v-if="store.queue.length === 0" style="text-align:center;color:var(--text-muted);padding:20px">队列为空</div>
      <div v-for="item in store.queue" :key="item.id" class="queue-item">
        <div class="info">
          <span>{{ item.song?.title || '未知歌曲' }}</span>
          <span v-if="item.song?.artist" style="color:var(--text-muted);font-size:0.85em"> - {{ item.song.artist }}</span>
          <div class="meta">
            <span :class="['badge', item.status === 'playing' ? 'badge-playing' : 'badge-pending']">
              {{ item.status === 'playing' ? '播放中' : '等待' }}
            </span>
            <span v-if="store.deviceUser?.role === 'admin'">点歌: {{ item.requested_by }}</span>
            <button
              v-if="store.deviceUser?.role === 'admin' && item.status !== 'playing'"
              class="btn btn-danger btn-small" style="margin-left:8px"
              @click="removeQueueItem(item.id)">移除</button>
          </div>
        </div>
      </div>
    </div>
    <div v-if="store.deviceUser?.role === 'admin'" class="card">
      <h2>📜 播放历史</h2>
      <div v-if="store.history.length === 0" style="text-align:center;color:var(--text-muted);padding:20px">暂无历史</div>
      <div style="max-height:300px;overflow-y:auto">
        <div v-for="h in store.history" :key="h.id" class="queue-item">
          <span>{{ h.song?.title || '未知' }}</span>
          <span style="color:var(--text-muted);font-size:0.8em">{{ h.requested_by || '' }} · {{ h.played_at || '' }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
