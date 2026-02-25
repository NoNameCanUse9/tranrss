<script setup lang="ts">
import { ref, computed } from 'vue'

interface QueueJob {
  id: number
  type: 'translate' | 'sync' | 'summarize'
  title: string
  subscription: string
  status: 'pending' | 'running' | 'done' | 'failed'
  progress: number
  startedAt: string
  duration: string | null
  error: string | null
}

const jobs = ref<QueueJob[]>([
  {
    id: 1,
    type: 'sync',
    title: '同步 Hacker News',
    subscription: 'Hacker News',
    status: 'running',
    progress: 65,
    startedAt: '00:15:32',
    duration: null,
    error: null,
  },
  {
    id: 2,
    type: 'translate',
    title: '翻译文章：The Future of AI',
    subscription: 'The Verge',
    status: 'running',
    progress: 30,
    startedAt: '00:15:40',
    duration: null,
    error: null,
  },
  {
    id: 3,
    type: 'translate',
    title: '翻译文章：OpenAI announces new model',
    subscription: 'Hacker News',
    status: 'pending',
    progress: 0,
    startedAt: '00:15:41',
    duration: null,
    error: null,
  },
  {
    id: 4,
    type: 'sync',
    title: '同步 MIT Technology Review',
    subscription: 'MIT Technology Review',
    status: 'failed',
    progress: 0,
    startedAt: '00:10:00',
    duration: '2s',
    error: '连接超时，无法获取订阅内容',
  },
  {
    id: 5,
    type: 'summarize',
    title: '生成摘要：Rust programming guide',
    subscription: 'Hacker News',
    status: 'done',
    progress: 100,
    startedAt: '00:05:00',
    duration: '8s',
    error: null,
  },
  {
    id: 6,
    type: 'translate',
    title: '翻译文章：Web3 technology overview',
    subscription: 'The Verge',
    status: 'done',
    progress: 100,
    startedAt: '00:03:00',
    duration: '12s',
    error: null,
  },
])

const filterStatus = ref<string>('all')

const statusOptions = [
  { title: '全部', value: 'all' },
  { title: '进行中', value: 'running' },
  { title: '等待中', value: 'pending' },
  { title: '已完成', value: 'done' },
  { title: '失败', value: 'failed' },
]

const filtered = computed(() => {
  if (filterStatus.value === 'all') return jobs.value
  return jobs.value.filter(j => j.status === filterStatus.value)
})

const stats = computed(() => ({
  running: jobs.value.filter(j => j.status === 'running').length,
  pending: jobs.value.filter(j => j.status === 'pending').length,
  done: jobs.value.filter(j => j.status === 'done').length,
  failed: jobs.value.filter(j => j.status === 'failed').length,
}))

const typeInfo = (type: string) => {
  const map: Record<string, { icon: string; color: string; label: string }> = {
    translate: { icon: 'mdi-translate', color: 'primary', label: '翻译' },
    sync: { icon: 'mdi-sync', color: 'secondary', label: '同步' },
    summarize: { icon: 'mdi-text-short', color: 'tertiary', label: '摘要' },
  }
  return map[type] ?? map.translate
}

const statusInfo = (status: string) => {
  const map: Record<string, { color: string; label: string; icon: string }> = {
    pending: { color: 'warning', label: '等待中', icon: 'mdi-clock-outline' },
    running: { color: 'primary', label: '进行中', icon: 'mdi-loading mdi-spin' },
    done: { color: 'success', label: '已完成', icon: 'mdi-check-circle-outline' },
    failed: { color: 'error', label: '失败', icon: 'mdi-alert-circle-outline' },
  }
  return map[status] ?? map.pending
}

const retryJob = async (job: QueueJob) => {
  job.status = 'pending'
  job.error = null
  job.progress = 0
  await new Promise(r => setTimeout(r, 800))
  job.status = 'running'
}

const clearCompleted = () => {
  jobs.value = jobs.value.filter(j => j.status !== 'done')
}
</script>

<template>
  <div class="queue-view">
    <div class="d-flex align-center justify-space-between mb-6">
      <div>
        <h2 class="text-h5 font-weight-bold">任务队列</h2>
        <p class="text-body-2 text-medium-emphasis mt-1">后台任务处理状态</p>
      </div>
      <v-btn
        v-if="stats.done > 0"
        variant="tonal"
        color="surface-variant"
        rounded="pill"
        class="text-none"
        @click="clearCompleted"
      >
        <v-icon start>mdi-broom</v-icon>
        清除已完成
      </v-btn>
    </div>

    <!-- 统计卡片 -->
    <v-row class="mb-6">
      <v-col v-for="stat in [
        { label: '进行中', count: stats.running, icon: 'mdi-progress-clock', color: 'primary' },
        { label: '等待中', count: stats.pending, icon: 'mdi-clock-outline', color: 'warning' },
        { label: '已完成', count: stats.done, icon: 'mdi-check-circle-outline', color: 'success' },
        { label: '失败', count: stats.failed, icon: 'mdi-alert-circle-outline', color: 'error' },
      ]" :key="stat.label" cols="6" sm="3">
        <v-card rounded="xl" variant="tonal" :color="stat.color" class="text-center pa-4">
          <v-icon :color="stat.color" size="28" class="mb-1">{{ stat.icon }}</v-icon>
          <p class="text-h5 font-weight-bold">{{ stat.count }}</p>
          <p class="text-caption text-medium-emphasis">{{ stat.label }}</p>
        </v-card>
      </v-col>
    </v-row>

    <!-- 筛选标签 -->
    <v-chip-group v-model="filterStatus" selected-class="bg-primary" class="mb-4" mandatory>
      <v-chip
        v-for="opt in statusOptions"
        :key="opt.value"
        :value="opt.value"
        variant="tonal"
        rounded="pill"
        class="text-none"
      >
        {{ opt.title }}
      </v-chip>
    </v-chip-group>

    <!-- 任务列表 -->
    <v-card v-if="filtered.length === 0" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-10">
      <v-icon size="48" color="primary" class="mb-3">mdi-clipboard-check-outline</v-icon>
      <p class="text-body-1">暂无任务</p>
    </v-card>

    <div v-else class="d-flex flex-column gap-3">
      <v-card
        v-for="job in filtered"
        :key="job.id"
        rounded="xl"
        variant="flat"
        color="surface-variant"
        class="job-card"
      >
        <v-card-text class="pa-5">
          <div class="d-flex align-start gap-3 mb-3">
            <!-- 类型图标 -->
            <v-avatar :color="typeInfo(job.type).color" size="36" rounded="lg">
              <v-icon color="white" size="18">{{ typeInfo(job.type).icon }}</v-icon>
            </v-avatar>

            <div class="flex-1 min-w-0">
              <div class="d-flex align-start justify-space-between gap-2">
                <p class="text-body-2 font-weight-semibold text-truncate flex-1">{{ job.title }}</p>
                <v-chip
                  :color="statusInfo(job.status).color"
                  size="x-small"
                  variant="tonal"
                  class="text-none flex-shrink-0"
                >
                  {{ statusInfo(job.status).label }}
                </v-chip>
              </div>
              <div class="d-flex align-center gap-3 mt-1 text-caption text-medium-emphasis">
                <span>
                  <v-icon size="12" class="mr-1">mdi-rss</v-icon>{{ job.subscription }}
                </span>
                <span>
                  <v-icon size="12" class="mr-1">mdi-clock-outline</v-icon>{{ job.startedAt }}
                  <span v-if="job.duration"> · {{ job.duration }}</span>
                </span>
                <v-chip :color="typeInfo(job.type).color" size="x-small" variant="text" class="text-none pa-0">
                  {{ typeInfo(job.type).label }}
                </v-chip>
              </div>
            </div>
          </div>

          <!-- 进度条 (running / pending) -->
          <v-progress-linear
            v-if="job.status === 'running'"
            :model-value="job.progress"
            color="primary"
            bg-color="surface-variant"
            rounded
            height="6"
            class="mb-2"
          />
          <v-progress-linear
            v-else-if="job.status === 'pending'"
            indeterminate
            color="warning"
            bg-color="surface-variant"
            rounded
            height="6"
            class="mb-2"
            :buffer-value="0"
          />

          <!-- 错误信息 -->
          <v-alert
            v-if="job.error"
            type="error"
            variant="tonal"
            density="compact"
            rounded="lg"
            class="mt-2 mb-2 text-caption"
          >
            {{ job.error }}
            <template #append>
              <v-btn size="x-small" variant="text" color="error" class="text-none" @click="retryJob(job)">
                重试
              </v-btn>
            </template>
          </v-alert>
        </v-card-text>
      </v-card>
    </div>
  </div>
</template>

<style scoped>
.job-card {
  transition: box-shadow 0.2s ease;
}
.job-card:hover {
  box-shadow: 0 4px 20px rgba(var(--v-theme-primary), 0.1) !important;
}
.gap-2 { gap: 8px; }
.gap-3 { gap: 12px; }
.min-w-0 { min-width: 0; }
</style>
