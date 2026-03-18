<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

interface JobData {
  feed_id?: number
  article_id?: number
  initiator_user_id?: number
  user_id?: number
}

interface BackendJob {
  id: string
  job_type: string
  status: string
  attempts: number
  last_error: string | null
  run_at: number
  done_at: number | null
  job_data: JobData
  title_label: string | null
}

interface SubJob {
  id: string
  title: string
  status: 'pending' | 'running' | 'done' | 'failed'
  error: string | null
}

interface QueueJob {
  id: string
  groupedIds: string[]
  type: 'translate' | 'sync' | 'summarize' | 'cron'
  titles: string[]
  title: string
  subscription: string
  status: 'pending' | 'running' | 'done' | 'failed'
  progress: number
  startedAt: string
  run_at: number
  duration: string | null
  error: string | null
  isGroup?: boolean
  expanded?: boolean
  subJobs?: SubJob[]
  stats?: {
    pending: number
    running: number
    done: number
    failed: number
  }
  articleId?: number
  feedId?: number
}

const jobs = ref<QueueJob[]>([])
let timer: any = null

const groupJobs = (rawJobs: any[]): QueueJob[] => {
  if (!rawJobs.length) return []
  const grouped: QueueJob[] = []
  
  
  for (const rj of rawJobs) {
    if (rj.type === 'cron') continue;

    if (rj.type === 'sync') {
      // Find latest sync group to see if we can merge (keep time proximity grouping for sync)
      const lastGroup = grouped.length > 0 ? grouped[grouped.length - 1] : null
      if (lastGroup && 
          lastGroup.isGroup && 
          lastGroup.type === 'sync' &&
          Math.abs(lastGroup.run_at - rj.run_at) <= 120) {
        
        lastGroup.groupedIds.push(rj.id)
        lastGroup.subJobs!.push({
           id: rj.id,
           title: rj.title,
           status: rj.status,
           error: rj.error
        })
        
        lastGroup.stats!.pending += rj.status === 'pending' ? 1 : 0
        lastGroup.stats!.running += rj.status === 'running' ? 1 : 0
        lastGroup.stats!.done += rj.status === 'done' ? 1 : 0
        lastGroup.stats!.failed += rj.status === 'failed' ? 1 : 0
        
        if (lastGroup.stats!.running > 0) lastGroup.status = 'running'
        else if (lastGroup.stats!.pending > 0) lastGroup.status = 'pending'
        else if (lastGroup.stats!.failed > 0) lastGroup.status = 'failed'
        else lastGroup.status = 'done'
        continue
      }
      
      // Create new sync group
      grouped.push({
        ...rj,
        title: '订阅更新任务',
        titles: [rj.title],
        groupedIds: [rj.id],
        isGroup: true,
        expanded: false,
        subJobs: [{
           id: rj.id,
           title: rj.title,
           status: rj.status,
           error: rj.error
        }],
        stats: {
           pending: rj.status === 'pending' ? 1 : 0,
           running: rj.status === 'running' ? 1 : 0,
           done: rj.status === 'done' ? 1 : 0,
           failed: rj.status === 'failed' ? 1 : 0,
        }
      })
      continue
    }

    if (rj.type === 'translate' || rj.type === 'summarize') {
      // Group by title (which should be the article title)
      const existingGroup = grouped.find(g => 
        g.isGroup && 
        g.type === rj.type && 
        g.title === rj.title &&
        (g.articleId === rj.articleId)
      )

      if (existingGroup) {
        existingGroup.groupedIds.push(rj.id)
        existingGroup.subJobs!.push({
           id: rj.id,
           title: rj.title,
           status: rj.status,
           error: rj.error
        })
        
        existingGroup.stats!.pending += rj.status === 'pending' ? 1 : 0
        existingGroup.stats!.running += rj.status === 'running' ? 1 : 0
        existingGroup.stats!.done += rj.status === 'done' ? 1 : 0
        existingGroup.stats!.failed += rj.status === 'failed' ? 1 : 0
        
        if (existingGroup.stats!.running > 0) existingGroup.status = 'running'
        else if (existingGroup.stats!.pending > 0) existingGroup.status = 'pending'
        else if (existingGroup.stats!.failed > 0) existingGroup.stats!.failed >= existingGroup.groupedIds.length ? existingGroup.status = 'failed' : existingGroup.status = 'done'
        else existingGroup.status = 'done'
        
        // Use earliest run_at and combine errors
        if (rj.run_at < existingGroup.run_at) existingGroup.run_at = rj.run_at
        continue
      }

      // Create new group for this article task
      grouped.push({
        ...rj,
        titles: [rj.title],
        groupedIds: [rj.id],
        isGroup: true,
        expanded: false,
        subJobs: [{
           id: rj.id,
           title: rj.title,
           status: rj.status,
           error: rj.error
        }],
        stats: {
           pending: rj.status === 'pending' ? 1 : 0,
           running: rj.status === 'running' ? 1 : 0,
           done: rj.status === 'done' ? 1 : 0,
           failed: rj.status === 'failed' ? 1 : 0,
        }
      })
      continue
    }
    
    // Fallback for other jobs or non-grouped scenarios
    grouped.push({
      ...rj,
      titles: [rj.title],
      groupedIds: [rj.id],
      isGroup: false,
      expanded: false,
      subJobs: [],
      stats: {
         pending: rj.status === 'pending' ? 1 : 0,
         running: rj.status === 'running' ? 1 : 0,
         done: rj.status === 'done' ? 1 : 0,
         failed: rj.status === 'failed' ? 1 : 0,
      }
    })
  }
  
  return grouped
}

const fetchJobs = async () => {
  try {
    const response = await fetch('/api/jobs', {
      headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
    })
    if (response.ok) {
      const data: BackendJob[] = await response.json()
      const mapped = data.map(mapBackendJob)
      jobs.value = groupJobs(mapped)
    }
  } catch (e) {
    console.error('Fetch jobs failed:', e)
  }
}

const mapBackendJob = (bj: BackendJob): QueueJob => {
  const typeMap: Record<string, 'translate' | 'sync' | 'summarize' | 'cron'> = {
    'tranrss_backend::services::jobs::SyncFeedJob': 'sync',
    'tranrss_backend::services::jobs::TranslateArticleJob': 'translate',
    'tranrss_backend::services::jobs::SummarizeArticleJob': 'summarize',
    'tranrss_backend::services::jobs::RefreshFeedsJob': 'cron',
  }

  const type = typeMap[bj.job_type] || 'sync'
  let rawStatus = bj.status.toLowerCase()
  let status: 'pending' | 'running' | 'done' | 'failed'
  
  if (['pending', 'retry'].includes(rawStatus)) {
    status = 'pending'
  } else if (rawStatus === 'running') {
    status = 'running'
  } else if (rawStatus === 'done') {
    status = 'done'
  } else {
    // Treat 'failed', 'dead', 'killed' etc as failed
    status = 'failed'
  }
  
  let title = '未知任务'
  let subscription = '-'

  if (bj.title_label) {
    title = bj.title_label
    subscription = type === 'cron' ? '系统自动化任务' : `ID: ${bj.job_data.feed_id || bj.job_data.article_id || '?'}`
  } else if (type === 'sync') {
    title = `同步源 #${bj.job_data.feed_id || '?'}`
    subscription = `Feed ID: ${bj.job_data.feed_id || '?'}`
  } else if (type === 'translate') {
    title = `翻译文章 (ID: ${bj.job_data.article_id || '?'})`
    subscription = `Article ID: ${bj.job_data.article_id || '?'}`
  } else if (type === 'summarize') {
    title = `生成文章摘要 (ID: ${bj.job_data.article_id || '?'})`
    subscription = `Article ID: ${bj.job_data.article_id || '?'}`
  } else if (type === 'cron') {
    title = '定时刷新全部订阅'
    subscription = '系统自动化任务'
  }

  const startDate = new Date(bj.run_at * 1000)
  const startedAt = startDate.toLocaleTimeString()
  
  let duration = null
  if (bj.done_at) {
    const seconds = bj.done_at - bj.run_at
    duration = seconds >= 60 ? `${Math.floor(seconds / 60)}m ${seconds % 60}s` : `${seconds}s`
  }

  return {
    id: bj.id,
    groupedIds: [bj.id],
    type,
    title: title,
    titles: [title],
    subscription,
    status,
    progress: status === 'done' ? 100 : (status === 'running' ? 50 : 0),
    startedAt,
    run_at: bj.run_at,
    duration,
    error: bj.last_error,
    articleId: bj.job_data.article_id,
    feedId: bj.job_data.feed_id,
  } as unknown as any // using 'any' mapping bridge for groupJobs
}

onMounted(() => {
  fetchJobs()
  timer = setInterval(fetchJobs, 30000)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
})

const filterStatus = ref<string>('all')
const filterType = ref<string>('all')

const statusOptions = [
  { title: '全部状态', value: 'all' },
  { title: '进行中', value: 'running' },
  { title: '等待中', value: 'pending' },
  { title: '已完成', value: 'done' },
  { title: '失败', value: 'failed' },
]

const typeOptions = [
  { title: '全部任务', value: 'all' },
  { title: '订阅更新', value: 'update' },
  { title: 'AI 服务', value: 'ai' },
]

const filtered = computed(() => {
  let result = jobs.value
  
  // 状态筛选
  if (filterStatus.value !== 'all') {
    result = result.filter(j => j.status === filterStatus.value)
  }
  
  // 类型分组筛选
  if (filterType.value === 'update') {
    result = result.filter(j => j.type === 'sync' || j.type === 'cron')
  } else if (filterType.value === 'ai') {
    result = result.filter(j => j.type === 'translate' || j.type === 'summarize')
  }
  
  return result
})

const stats = computed(() => ({
  running: jobs.value.filter(j => j.status === 'running').length,
  pending: jobs.value.filter(j => j.status === 'pending').length,
  done: jobs.value.filter(j => j.status === 'done').length,
  failed: jobs.value.filter(j => j.status === 'failed').length,
}))

const typeInfo = (type: string): { icon: string; color: string; label: string } => {
  const map: Record<string, { icon: string; color: string; label: string }> = {
    translate: { icon: 'mdi-translate', color: 'primary', label: '翻译' },
    sync: { icon: 'mdi-sync', color: 'secondary', label: '同步' },
    summarize: { icon: 'mdi-text-short', color: 'tertiary', label: '摘要' },
    cron: { icon: 'mdi-calendar-clock', color: 'info', label: '定时' },
  }
  return (map[type] ?? map.translate)!
}

const statusInfo = (status: string): { color: string; label: string; icon: string } => {
  const map: Record<string, { color: string; label: string; icon: string }> = {
    pending: { color: 'warning', label: '等待中', icon: 'mdi-clock-outline' },
    running: { color: 'primary', label: '进行中', icon: 'mdi-loading mdi-spin' },
    done: { color: 'success', label: '已完成', icon: 'mdi-check-circle-outline' },
    failed: { color: 'error', label: '已停止(重试上限)', icon: 'mdi-alert-circle-outline' },
  }
  return (map[status] ?? map.pending)!
}

const retryJob = async (job: QueueJob) => {
  try {
    for (const id of job.groupedIds) {
      await fetch(`/api/jobs/${id}/retry`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
      })
    }
    fetchJobs()
  } catch (e) {
    console.error('Retry failed:', e)
  }
}

const clearCompleted = async () => {
  try {
    const res = await fetch(`/api/jobs/clear_completed`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
    })
    if (res.ok) {
      fetchJobs()
    }
  } catch (e) {
    console.error('Clear failed:', e)
  }
}
</script>

<template>
  <div class="queue-view">
    <div class="d-flex align-center justify-space-between mb-6">
      <div>
        <h2 class="text-h5 font-weight-bold">运行日志</h2>
        <p class="text-body-2 text-medium-emphasis mt-1">后台执行记录与日志详情</p>
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

    <!-- 统计卡片 (自动均分的 Grid 布局) -->
    <div class="stats-grid mb-6">
      <v-card v-for="stat in [
        { label: '进行中', count: stats.running, icon: 'mdi-progress-clock', color: 'primary' },
        { label: '等待中', count: stats.pending, icon: 'mdi-clock-outline', color: 'warning' },
        { label: '已完成', count: stats.done, icon: 'mdi-check-circle-outline', color: 'success' },
        { label: '失败', count: stats.failed, icon: 'mdi-alert-circle-outline', color: 'error' },
      ]" :key="stat.label" rounded="xl" variant="flat" color="surface" class="text-center pa-5 border-thin w-100 h-100">
        <div class="d-flex align-center justify-center mb-3">
           <v-avatar :color="`${stat.color}-lighten-4`" size="48">
             <v-icon :color="stat.color" size="24">{{ stat.icon }}</v-icon>
           </v-avatar>
        </div>
        <p class="text-h4 font-weight-bold mb-1">{{ stat.count }}</p>
        <p class="text-caption text-medium-emphasis text-uppercase font-weight-medium">{{ stat.label }}</p>
      </v-card>
    </div>

    <!-- 筛选工具栏 -->
    <div class="d-flex flex-wrap align-center gap-4 mb-4">
      <!-- 任务分类 -->
      <v-chip-group v-model="filterType" selected-class="bg-secondary" mandatory>
        <v-chip
          v-for="opt in typeOptions"
          :key="opt.value"
          :value="opt.value"
          variant="tonal"
          rounded="pill"
          class="text-none"
        >
          {{ opt.title }}
        </v-chip>
      </v-chip-group>

      <v-divider vertical class="mx-2 my-2" />

      <!-- 任务状态 -->
      <v-chip-group v-model="filterStatus" selected-class="bg-primary" mandatory>
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
    </div>

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
        :class="{ 'cursor-pointer': job.isGroup }"
        @click="job.isGroup && (job.expanded = !job.expanded)"
      >
        <v-card-text class="pa-5">
          <div class="d-flex align-start gap-4 mb-3">
            <!-- 类型图标 -->
            <v-avatar :color="typeInfo(job.type).color" size="36" rounded="lg">
              <v-icon color="white" size="18">{{ typeInfo(job.type).icon }}</v-icon>
            </v-avatar>

            <div class="flex-1 min-w-0">
              <div class="d-flex align-center justify-space-between gap-3">
                <div v-if="job.isGroup" class="d-flex align-center gap-4 flex-wrap flex-1 min-w-0">
                  <span class="text-h6 font-weight-bold mr-4 text-truncate" style="max-width: 50%;">{{ job.title }}</span>
                  <v-chip size="x-small" variant="tonal" class="mr-2">{{ job.type === 'sync' ? `${job.groupedIds.length}个源` : `${job.groupedIds.length}个片段` }}</v-chip>
                  <div class="d-flex align-center gap-4">
                    <span v-if="job.stats?.done" class="d-flex align-center text-success text-body-2 font-weight-medium">
                      <v-icon size="16" class="mr-1">mdi-check-circle</v-icon>成功: {{ job.stats.done }}
                    </span>
                    <span v-if="job.stats?.failed" class="d-flex align-center text-error text-body-2 font-weight-medium">
                      <v-icon size="16" class="mr-1">mdi-alert-circle</v-icon>失败: {{ job.stats.failed }}
                    </span>
                    <span v-if="job.stats?.pending" class="d-flex align-center text-warning text-body-2 font-weight-medium">
                      <v-icon size="16" class="mr-1">mdi-clock-outline</v-icon>等待: {{ job.stats.pending }}
                    </span>
                    <span v-if="job.stats?.running" class="d-flex align-center text-primary text-body-2 font-weight-medium">
                      <v-icon size="16" class="mr-1">mdi-loading mdi-spin</v-icon>进行中: {{ job.stats.running }}
                    </span>
                  </div>
                </div>
                <div v-else class="text-body-2 font-weight-semibold flex-1 min-w-0" style="word-break: break-all;">
                  <span class="d-block text-truncate pb-1">{{ job.title }}</span>
                </div>
                
                <v-chip
                  v-if="!job.isGroup"
                  :color="statusInfo(job.status).color"
                  size="x-small"
                  variant="tonal"
                  class="text-none flex-shrink-0"
                >
                  {{ statusInfo(job.status).label }}
                </v-chip>
                <div v-else class="flex-shrink-0 d-flex align-center">
                  <v-btn v-if="job.stats?.failed" size="small" variant="tonal" @click.stop="retryJob({ groupedIds: job.subJobs?.filter(s => s.status === 'failed').map(s => s.id) || [] } as any)" color="error" class="px-3 mr-4 text-none rounded-pill">
                    <v-icon start>mdi-refresh</v-icon>重试失败
                  </v-btn>
                  <v-icon color="medium-emphasis" size="24">{{ job.expanded ? 'mdi-chevron-up' : 'mdi-chevron-down' }}</v-icon>
                </div>
              </div>
              
              <div class="d-flex align-center gap-3 mt-2 text-caption text-medium-emphasis">
                <span v-if="!job.isGroup" class="d-flex align-center">
                  <v-icon size="14" class="mr-1">mdi-identifier</v-icon>{{ job.subscription }}
                </span>
                <span v-else class="d-flex align-center">
                  <v-icon size="14" class="mr-1">mdi-format-list-bulleted-type</v-icon>{{ job.type === 'sync' ? '订阅更新聚合任务' : '文章翻译聚合任务' }}
                </span>
                
                <span class="d-flex align-center">
                  <v-icon size="14" class="mr-1">mdi-clock-outline</v-icon>{{ job.startedAt }}
                  <v-tooltip v-if="job.duration" activator="parent" location="top">耗时 {{ job.duration }}</v-tooltip>
                </span>
                <v-chip :color="typeInfo(job.type).color" size="x-small" variant="tonal" class="text-none">
                  {{ typeInfo(job.type).label }}
                </v-chip>
              </div>
            </div>
          </div>

          <!-- 进度条 (running / pending) -->
          <v-progress-linear
            v-if="job.status === 'running' && !job.isGroup"
            :model-value="job.progress"
            color="primary"
            bg-color="surface-variant"
            rounded
            height="6"
            class="mb-2 wavy-progress"
          />
          <v-progress-linear
            v-else-if="job.status === 'pending' && !job.isGroup"
            indeterminate
            color="warning"
            bg-color="surface-variant"
            rounded
            height="6"
            class="mb-2"
            :buffer-value="0"
          />

          <!-- 错误信息 / 分组详情展出 -->
          <v-alert
            v-if="!job.isGroup && job.error"
            type="error"
            variant="tonal"
            density="compact"
            rounded="lg"
            class="mt-2 mb-2 text-caption"
          >
            {{ job.error }}
            <template #append>
              <v-btn size="x-small" variant="text" color="error" class="text-none" @click="retryJob({ groupedIds: [job.id] } as any)">
                重试
              </v-btn>
            </template>
          </v-alert>

          <v-expand-transition>
            <div v-if="job.isGroup && job.expanded" class="pt-3 d-flex flex-column gap-2">
              <div v-for="sub in job.subJobs" :key="sub.id" class="d-flex align-center justify-space-between bg-surface pa-2 rounded-lg border-thin">
                <div class="d-flex align-center gap-2 min-w-0 flex-1">
                  <v-icon :color="statusInfo(sub.status).color" size="18">{{ statusInfo(sub.status).icon }}</v-icon>
                  <span class="text-caption text-truncate font-weight-medium">{{ sub.title }}</span>
                </div>
                <div class="d-flex align-center gap-2">
                  <v-tooltip v-if="sub.error" text="查看错误" location="top">
                    <template v-slot:activator="{ props }">
                      <v-btn v-bind="props" icon variant="text" size="small" color="error" density="compact">
                        <v-icon size="16">mdi-alert-circle</v-icon>
                      </v-btn>
                    </template>
                    <span>{{ sub.error }}</span>
                  </v-tooltip>
                  <v-btn v-if="sub.status === 'failed'" icon variant="text" size="small" color="primary" density="compact" @click.stop="retryJob({ groupedIds: [sub.id] } as any)">
                    <v-icon size="16">mdi-refresh</v-icon>
                  </v-btn>
                </div>
              </div>
            </div>
          </v-expand-transition>
        </v-card-text>
      </v-card>
    </div>
  </div>
</template>

<style scoped>
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
  width: 100%;
}
.job-card {
  transition: 
    transform var(--md-motion-duration-medium) var(--md-motion-easing-emphasized),
    box-shadow var(--md-motion-duration-medium) var(--md-motion-easing-emphasized) !important;
}
.job-card:active {
  transform: scale(0.98);
  transition-duration: 100ms !important;
}
.job-card:hover {
  box-shadow: 0 4px 20px rgba(var(--v-theme-primary), 0.1) !important;
  transform: translateY(-2px);
}
.gap-2 { gap: 8px; }
.gap-3 { gap: 12px; }
.gap-4 { gap: 16px; }
.gap-5 { gap: 20px; }
.min-w-0 { min-width: 0; }
.cursor-pointer { cursor: pointer; }

/* Google Play / Material 3 风格的线性进度条 */
.wavy-progress :deep(.v-progress-linear__indeterminate) {
  background: none !important;
}

.wavy-progress :deep(.v-progress-linear__indeterminate)::before {
  content: "";
  position: absolute;
  background-color: inherit;
  top: 0; left: 0; bottom: 0;
  will-change: left, right;
  animation: m3-linear-1 2.1s cubic-bezier(0.65, 0.815, 0.735, 0.395) infinite;
}

.wavy-progress :deep(.v-progress-linear__indeterminate)::after {
  content: "";
  position: absolute;
  background-color: inherit;
  top: 0; left: 0; bottom: 0;
  will-change: left, right;
  animation: m3-linear-2 2.1s cubic-bezier(0.165, 0.84, 0.44, 1) infinite;
  animation-delay: 1.15s;
}

@keyframes m3-linear-1 {
  0% { left: -35%; right: 100%; }
  60% { left: 100%; right: -90%; }
  100% { left: 100%; right: -90%; }
}

@keyframes m3-linear-2 {
  0% { left: -200%; right: 100%; }
  60% { left: 107%; right: -8%; }
  100% { left: 107%; right: -8%; }
}

/* 针对 determinate 状态添加流光 */
.wavy-progress :deep(.v-progress-linear__determinate)::after {
  content: "";
  position: absolute;
  top: 0; left: 0; bottom: 0; right: 0;
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0) 0%,
    rgba(255, 255, 255, 0.4) 50%,
    rgba(255, 255, 255, 0) 100%
  );
  animation: wave-animation 2s infinite linear;
}

@keyframes wave-animation {
  from { transform: translateX(-100%); }
  to { transform: translateX(100%); }
}
</style>
