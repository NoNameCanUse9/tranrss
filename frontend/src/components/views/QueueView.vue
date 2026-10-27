<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  mdiTranslate,
  mdiSync,
  mdiTextShort,
  mdiCalendarClock,
  mdiClockOutline,
  mdiLoading,
  mdiCheckCircleOutline,
  mdiAlertCircleOutline,
  mdiBroom,
  mdiProgressClock,
  mdiClipboardCheckOutline,
  mdiCheckCircle,
  mdiAlertCircle,
  mdiRefresh,
  mdiChevronUp,
  mdiChevronDown,
  mdiIdentifier,
  mdiFormatListBulletedType
} from '@mdi/js'
import { apiFetch } from '../../utils/api'

const { t } = useI18n()

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
  job_data: JobData | null
  title_label: string | null
  feed_id: number | null
  feed_title: string | null
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
  errors?: string[]
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
  feedTitle?: string
}

const loading = ref(true)
const error = ref('')
let requestSeq = 0

const aggregateStatus = (stats: NonNullable<QueueJob['stats']>): QueueJob['status'] => {
  if (stats.running > 0) return 'running'
  if (stats.pending > 0) return 'pending'
  if (stats.failed > 0) return 'failed'
  return 'done'
}

const MAX_ERRORS_PER_GROUP = 5
// 单条错误可能是一整页上游 HTML 或调用栈；截断后再进 DOM，避免卡片被撑爆
const MAX_ERROR_CHARS = 300

const clipError = (message: string): string => {
  const normalized = message.replace(/\s+/g, ' ').trim()
  return normalized.length > MAX_ERROR_CHARS
    ? normalized.slice(0, MAX_ERROR_CHARS) + '…'
    : normalized
}

// Jobs arrive newest-first, so keep the leading (latest) distinct messages per group.
const mergeErrors = (errors: string[], next: string | null | undefined): string[] => {
  if (!next) return errors
  const message = clipError(next)
  if (!message || errors.includes(message)) return errors
  errors.push(message)
  if (errors.length > MAX_ERRORS_PER_GROUP) errors.length = MAX_ERRORS_PER_GROUP
  return errors
}
const jobs = ref<QueueJob[]>([])
let timer: any = null

const groupJobs = (rawJobs: any[], expandedKeys: Set<string> = new Set()): QueueJob[] => {
  if (!rawJobs.length) return []
  const grouped: QueueJob[] = []
  
  
  for (const rj of rawJobs) {

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
        lastGroup.status = aggregateStatus(lastGroup.stats!)
        if (rj.title) lastGroup.titles.push(rj.title)
        lastGroup.errors = mergeErrors(lastGroup.errors ?? [], rj.error)
        continue
      }
      
      // Create new sync group
      grouped.push({
        ...rj,
        title: t('queue.sync_task'),
        titles: [rj.title],
        groupedIds: [rj.id],
        isGroup: true,
        expanded: false,
        errors: mergeErrors([], rj.error),
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
      // Group by feedId if available, fallback to title (article title)
      const existingGroup = grouped.find(g => 
        g.isGroup && 
        g.type === rj.type && 
        (rj.feedId ? g.feedId === rj.feedId : g.title === rj.title)
      )

      if (existingGroup) {
        existingGroup.groupedIds.push(rj.id)
        existingGroup.titles.push(rj.title)
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
        existingGroup.status = aggregateStatus(existingGroup.stats!)
        
        // Use earliest run_at and merge distinct errors
        if (rj.run_at < existingGroup.run_at) existingGroup.run_at = rj.run_at
        existingGroup.errors = mergeErrors(existingGroup.errors ?? [], rj.error)
        continue
      }

      // Create new group for this feed/article task
      grouped.push({
        ...rj,
        title: rj.feedTitle || rj.title, // Use feed title as card title
        titles: [rj.title],
        groupedIds: [rj.id],
        isGroup: true,
        expanded: false,
        errors: mergeErrors([], rj.error),
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
      errors: mergeErrors([], rj.error),
      subJobs: [],
      stats: {
         pending: rj.status === 'pending' ? 1 : 0,
         running: rj.status === 'running' ? 1 : 0,
         done: rj.status === 'done' ? 1 : 0,
         failed: rj.status === 'failed' ? 1 : 0,
      }
    })
  }

  for (const group of grouped) {
    if (group.isGroup) {
      group.error = group.errors?.[0] ?? null
      group.expanded = expandedKeys.has(group.groupedIds[0] ?? group.id)
    }
  }
  
  return grouped
}

const fetchJobs = async () => {
  const seq = ++requestSeq
  loading.value = true
  try {
    const response = await apiFetch('/api/jobs')
    if (seq !== requestSeq) return
    if (!response.ok) throw new Error(t('queue.load_failed'))
    const data: BackendJob[] = await response.json()
    if (seq !== requestSeq) return
    const mapped = data.map(mapBackendJob)
    const expandedKeys = new Set(
      jobs.value
        .filter(job => job.isGroup && job.expanded)
        .map(job => job.groupedIds[0] ?? job.id)
    )
    jobs.value = groupJobs(mapped, expandedKeys)
    error.value = ''
  } catch (e: any) {
    if (seq !== requestSeq) return
    error.value = e?.message || t('queue.load_failed')
  } finally {
    if (seq === requestSeq) loading.value = false
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
  
  let title = t('queue.empty')
  let subscription = '-'

  if (bj.title_label) {
    title = bj.title_label
    subscription = type === 'cron' ? t('queue.job_cron') : `ID: ${bj.job_data?.feed_id || bj.job_data?.article_id || '?'}`
  } else if (type === 'sync') {
    title = `${t('queue.job_sync')} #${bj.job_data?.feed_id || '?'}`
    subscription = `Feed ID: ${bj.job_data?.feed_id || '?'}`
  } else if (type === 'translate') {
    title = `${t('queue.job_translate')} (ID: ${bj.job_data?.article_id || '?'})`
    subscription = `Article ID: ${bj.job_data?.article_id || '?'}`
  } else if (type === 'summarize') {
    title = `${t('queue.job_summarize')} (ID: ${bj.job_data?.article_id || '?'})`
    subscription = `Article ID: ${bj.job_data?.article_id || '?'}`
  } else if (type === 'cron') {
    title = t('queue.job_cron')
    subscription = t('queue.job_cron')
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
    articleId: bj.job_data?.article_id,
    feedId: bj.feed_id || bj.job_data?.feed_id,
    feedTitle: bj.feed_title,
  } as unknown as any // using 'any' mapping bridge for groupJobs
}

let eventSource: EventSource | null = null

interface SystemLog {
  id: number
  level: string
  target: string
  message: string
  time: string
}

let nextLogId = 0
const systemLogs = ref<SystemLog[]>([])

let refreshTimer: ReturnType<typeof setTimeout> | null = null
const scheduleRefresh = () => {
  if (refreshTimer) clearTimeout(refreshTimer)
  refreshTimer = setTimeout(() => {
    refreshTimer = null
    fetchJobs()
  }, 400)
}

onMounted(() => {
  fetchJobs()
  // 保留低频轮询作为 SSE 断线的兜底
  timer = setInterval(fetchJobs, 60000)

  // 监听 SSE 事件以实现近实时更新
  eventSource = new EventSource('/api/events')
  eventSource.onmessage = (event) => {
    const msg = event.data
    // 当有 feed 刷新或文章更新完成时，延迟拉取最新 job 状态，避免事件风暴
    if (msg === 'REFRESH_FEEDS' || msg.startsWith('ARTICLE_UPDATED:')) {
      scheduleRefresh()
    }
    // 捕获后台推送的 WARN+ 日志
    if (msg.startsWith('LOG:')) {
      try {
        const log = JSON.parse(msg.slice(4))
        systemLogs.value.unshift({
          ...log,
          time: new Date().toLocaleTimeString(),
          id: ++nextLogId
        })
        // 只保留最近 50 条
        if (systemLogs.value.length > 50) systemLogs.value.pop()
      } catch {}
    }
  }
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
  if (refreshTimer) clearTimeout(refreshTimer)
  eventSource?.close()
})

const filterStatus = ref<string>('all')
const filterType = ref<string>('all')

const statusOptions = computed(() => [
  { title: t('queue.filter_all_status'), value: 'all' },
  { title: t('queue.stat_running'), value: 'running' },
  { title: t('queue.stat_pending'), value: 'pending' },
  { title: t('queue.stat_done'), value: 'done' },
  { title: t('queue.stat_failed'), value: 'failed' },
])

const typeOptions = computed(() => [
  { title: t('queue.filter_all_types'), value: 'all' },
  { title: t('queue.filter_sync'), value: 'update' },
  { title: t('queue.filter_ai'), value: 'ai' },
  { title: t('queue.filter_system'), value: 'system' },
])

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
  } else if (filterType.value === 'system') {
    result = result.filter(j => j.type === 'cron')
  }
  
  return result
})

const stats = computed(() => ({
  running: jobs.value.filter(j => j.status === 'running').length,
  pending: jobs.value.filter(j => j.status === 'pending').length,
  done: jobs.value.filter(j => j.status === 'done').length,
  failed: jobs.value.filter(j => j.status === 'failed').length,
}))

const groupProgress = (job: QueueJob) => {
  const s = job.stats
  if (!s) return 0
  const total = s.pending + s.running + s.done + s.failed
  return total === 0 ? 0 : Math.round((s.done / total) * 100)
}

const typeInfo = (type: string): { icon: string; color: string; label: string } => {
  const map: Record<string, { icon: string; color: string; label: string }> = {
    translate: { icon: mdiTranslate, color: 'primary', label: t('queue.job_translate') },
    sync: { icon: mdiSync, color: 'secondary', label: t('queue.job_sync') },
    summarize: { icon: mdiTextShort, color: 'tertiary', label: t('queue.job_summarize') },
    cron: { icon: mdiCalendarClock, color: 'info', label: t('queue.job_cron') },
  }
  return (map[type] ?? map.translate)!
}

const statusInfo = (status: string): { color: string; label: string; icon: string } => {
  const map: Record<string, { color: string; label: string; icon: string }> = {
    pending: { color: 'warning', label: t('queue.stat_pending'), icon: mdiClockOutline },
    running: { color: 'primary', label: t('queue.stat_running'), icon: mdiLoading },
    done: { color: 'success', label: t('queue.stat_done'), icon: mdiCheckCircleOutline },
    failed: { color: 'error', label: t('queue.stat_failed'), icon: mdiAlertCircleOutline },
  }
  return (map[status] ?? map.pending)!
}

const retryJob = async (job: QueueJob) => {
  try {
    for (const id of job.groupedIds) {
      await apiFetch(`/api/jobs/${id}/retry`, {
        method: 'POST'
      })
    }
    fetchJobs()
  } catch (e) {
    console.error('Retry failed:', e)
  }
}

const clearCompleted = async () => {
  try {
    const res = await apiFetch(`/api/jobs/clear_completed`, {
      method: 'POST'
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
    <div class="d-flex align-center justify-space-between flex-wrap gap-3 mb-6">
      <div class="min-w-0">
        <h2 class="text-h5 font-weight-bold">{{ $t('queue.title') }}</h2>
        <p class="text-body-2 text-medium-emphasis mt-1">{{ $t('queue.subtitle') }}</p>
      </div>
      <div class="d-flex align-center gap-2">
        <v-btn
          variant="tonal"
          color="primary"
          rounded="pill"
          class="text-none"
          :loading="loading"
          @click="fetchJobs"
        >
          <v-icon start :icon="mdiRefresh" />
          {{ $t('queue.refresh') }}
        </v-btn>
        <v-btn
          v-if="stats.done > 0"
          variant="tonal"
          color="surface-variant"
          rounded="pill"
          class="text-none"
          @click="clearCompleted"
        >
          <v-icon start :icon="mdiBroom" />
          {{ $t('queue.clear_done') }}
        </v-btn>
      </div>
    </div>

    <!-- 统计卡片 (自动均分的 Grid 布局) -->
    <div class="stats-grid mb-6">
      <v-card v-for="stat in [
        { label: $t('queue.stat_running'), count: stats.running, icon: mdiProgressClock, color: 'primary' },
        { label: $t('queue.stat_pending'), count: stats.pending, icon: mdiClockOutline, color: 'warning' },
        { label: $t('queue.stat_done'), count: stats.done, icon: mdiCheckCircleOutline, color: 'success' },
        { label: $t('queue.stat_failed'), count: stats.failed, icon: mdiAlertCircleOutline, color: 'error' },
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

      <v-divider vertical class="mx-2 my-2 filter-divider" />

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

    <!-- 加载状态 -->
    <v-card v-if="loading && jobs.length === 0" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-10">
      <v-progress-circular indeterminate color="primary" size="48" class="mb-4" />
      <p class="text-body-1">{{ $t('queue.loading') }}</p>
    </v-card>

    <!-- 加载失败 -->
    <v-card v-else-if="error" rounded="xl" variant="tonal" color="error" class="text-center pa-10">
      <v-icon size="48" color="error" class="mb-3">{{ mdiAlertCircleOutline }}</v-icon>
      <p class="text-body-1 mb-4">{{ error }}</p>
      <v-btn color="primary" rounded="pill" class="text-none" @click="fetchJobs">
        <v-icon start :icon="mdiRefresh" />
        {{ $t('common.retry') }}
      </v-btn>
    </v-card>

    <!-- 任务列表 -->
    <v-card v-else-if="filtered.length === 0" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-10">
      <v-icon size="48" color="primary" class="mb-3">{{ mdiClipboardCheckOutline }}</v-icon>
      <p class="text-body-1">{{ $t('queue.empty') }}</p>
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
          <div class="d-flex align-start gap-3 mb-3">
            <!-- 类型图标 -->
            <v-avatar :color="typeInfo(job.type).color" size="36" rounded="lg" class="flex-shrink-0">
              <v-icon color="white" size="18">{{ typeInfo(job.type).icon }}</v-icon>
            </v-avatar>

            <div class="flex-1 min-w-0">
              <!-- 第一行：标题和主要的聚合统计 -->
              <div class="d-flex flex-wrap align-center gap-2 mb-1 min-w-0">
                <span class="text-h6 font-weight-bold text-truncate job-title">{{ job.title }}</span>
                <v-chip size="x-small" variant="tonal" class="flex-shrink-0">{{ job.type === 'sync' ? `${job.groupedIds.length}${ $t('queue.unit_source') }` : `${job.titles.length}${ $t('queue.unit_article') }` }}</v-chip>

                <div v-if="job.isGroup" class="d-flex align-center gap-4 flex-shrink-0 job-stats">
                  <span v-if="job.stats?.done" class="d-flex align-center text-success text-body-2 font-weight-medium">
                    <v-icon size="16" class="mr-1">{{ mdiCheckCircle }}</v-icon>{{ job.stats.done }}
                  </span>
                  <span v-if="job.stats?.failed" class="d-flex align-center text-error text-body-2 font-weight-medium">
                    <v-icon size="16" class="mr-1">{{ mdiAlertCircle }}</v-icon>{{ job.stats.failed }}
                  </span>
                  <span v-if="job.stats?.pending" class="d-flex align-center text-warning text-body-2 font-weight-medium">
                    <v-icon size="16" class="mr-1">{{ mdiClockOutline }}</v-icon>{{ job.stats.pending }}
                  </span>
                  <span v-if="job.stats?.running" class="d-flex align-center text-primary text-body-2 font-weight-medium">
                    <v-icon size="16" class="mr-1 mdi-spin">{{ mdiLoading }}</v-icon>{{ job.stats.running }}
                  </span>
                </div>
              </div>

              <!-- 第二行：详细信息、类型及可能的错误摘要 -->
              <div class="d-flex flex-wrap align-center gap-3 text-caption text-medium-emphasis min-w-0">
                <span v-if="!job.isGroup" class="d-flex align-center flex-shrink-0">
                  <v-icon size="14" class="mr-1">{{ mdiIdentifier }}</v-icon>{{ job.subscription }}
                </span>
                <span v-else class="d-flex align-center flex-shrink-0">
                  <v-icon size="14" class="mr-1">{{ mdiFormatListBulletedType }}</v-icon>{{ job.type === 'sync' ? $t('queue.sync_agg') : $t('queue.trans_agg') }}
                </span>
                
                <span class="d-flex align-center flex-shrink-0">
                  <v-icon size="14" class="mr-1">{{ mdiClockOutline }}</v-icon>{{ job.startedAt }}
                </span>
                <span v-if="job.duration" class="d-flex align-center flex-shrink-0">
                  <v-icon size="14" class="mr-1">{{ mdiProgressClock }}</v-icon>{{ $t('queue.duration', { d: job.duration }) }}
                </span>
                <v-chip :color="typeInfo(job.type).color" size="x-small" variant="tonal" class="text-none flex-shrink-0">
                  {{ typeInfo(job.type).label }}
                </v-chip>
                <span v-if="job.error" class="text-error text-truncate job-error">({{ job.error }}<template v-if="(job.errors?.length ?? 0) > 1"> +{{ job.errors!.length - 1 }}</template>)</span>
              </div>
            </div>

            <!-- 最右侧动作：绝对靠右对齐 -->
            <div class="flex-shrink-0 d-flex align-center gap-1 job-actions">
              <v-chip
                v-if="!job.isGroup"
                :color="statusInfo(job.status).color"
                size="x-small"
                variant="tonal"
                class="text-none"
              >
                {{ statusInfo(job.status).label }}
              </v-chip>
              <div v-else class="d-flex align-center">
                <v-btn v-if="job.stats?.failed" size="small" variant="tonal" @click.stop="retryJob({ groupedIds: job.subJobs?.filter(s => s.status === 'failed').map(s => s.id) || [] } as any)" color="error" class="px-3 mr-4 text-none rounded-pill">
                  <v-icon start :icon="mdiRefresh" />{{ $t('queue.retry_failed') }}
                </v-btn>
                <v-icon color="medium-emphasis" size="28">{{ job.expanded ? mdiChevronUp : mdiChevronDown }}</v-icon>
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
          <v-progress-linear
            v-else-if="job.isGroup && (job.stats?.pending || job.stats?.running)"
            :model-value="groupProgress(job)"
            :color="job.status === 'failed' ? 'error' : 'primary'"
            bg-color="surface-variant"
            rounded
            height="6"
            class="mb-2"
            :class="{ 'wavy-progress': job.status === 'running' }"
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
            <div class="wrap-text">{{ job.error }}</div>
            <template #append>
              <v-btn size="x-small" variant="text" color="error" class="text-none" @click="retryJob({ groupedIds: [job.id] } as any)">
                {{ $t('common.retry') }}
              </v-btn>
            </template>
          </v-alert>

          <v-expand-transition>
            <div v-if="job.isGroup && job.expanded" class="pt-3 d-flex flex-column gap-2">
              <div v-if="job.errors?.length" class="d-flex flex-column gap-1">
                <span v-for="err in job.errors" :key="err" class="text-caption text-error wrap-text">• {{ err }}</span>
              </div>
              <div v-for="sub in job.subJobs" :key="sub.id" class="d-flex align-center justify-space-between bg-surface pa-2 rounded-lg border-thin">
                <div class="d-flex align-center gap-2 min-w-0 flex-1">
                  <v-icon :color="statusInfo(sub.status).color" size="18" :class="{ 'mdi-spin': sub.status === 'running' }" class="flex-shrink-0">{{ statusInfo(sub.status).icon }}</v-icon>
                  <span class="text-caption text-truncate font-weight-medium flex-1">{{ sub.title }}</span>
                </div>
                <div class="d-flex align-center gap-2 flex-shrink-0">
                  <v-tooltip v-if="sub.error" :text="$t('queue.view_error')" location="top">
                    <template v-slot:activator="{ props }">
                      <v-btn v-bind="props" icon variant="text" size="small" color="error" density="compact">
                        <v-icon size="16">{{ mdiAlertCircle }}</v-icon>
                      </v-btn>
                    </template>
                    <span>{{ sub.error }}</span>
                  </v-tooltip>
                  <v-btn v-if="sub.status === 'failed'" icon variant="text" size="small" color="primary" density="compact" @click.stop="retryJob({ groupedIds: [sub.id] } as any)">
                    <v-icon size="16">{{ mdiRefresh }}</v-icon>
                  </v-btn>
                </div>
              </div>
            </div>
          </v-expand-transition>
        </v-card-text>
      </v-card>
    </div>

    <!-- 系统日志面板 -->
    <v-card
      v-if="systemLogs.length > 0"
      rounded="xl" variant="flat" color="surface-variant"
      class="mt-6"
    >
      <v-card-title class="d-flex align-center">
        <v-icon start>{{ mdiAlertCircleOutline }}</v-icon>
        {{ $t('queue.system_logs') }} ({{ systemLogs.length }})
      </v-card-title>
      <v-card-text class="pa-0">
        <v-list density="compact" class="pa-0 bg-transparent">
          <v-list-item
            v-for="log in systemLogs" :key="log.id"
            class="text-caption log-item"
          >
            <template #prepend>
              <v-chip
                :color="log.level === 'ERROR' ? 'error' : 'warning'"
                size="x-small" variant="tonal" class="mr-2 log-level"
              >{{ log.level }}</v-chip>
            </template>
            <v-list-item-title class="text-caption font-weight-medium log-message">{{ log.message }}</v-list-item-title>
            <v-list-item-subtitle class="log-meta">{{ log.target }} · {{ log.time }}</v-list-item-subtitle>
          </v-list-item>
        </v-list>
      </v-card-text>
    </v-card>
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
  /* 任务条数由 user_setting.log_num_limit 决定（默认 300，可调大）。
     离屏卡片跳过布局/绘制，避免列表随任务累积越来越慢。 */
  content-visibility: auto;
  contain-intrinsic-size: auto 160px;
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
.wrap-text {
  white-space: pre-wrap !important;
  overflow-wrap: anywhere !important;
  word-break: normal !important;
}
.cursor-pointer { cursor: pointer; }

/* 任务卡片：标题与错误信息在窄屏下换行而不是被压扁 */
.job-title {
  flex: 1 1 240px;
  min-width: 0;
}

.job-error {
  flex: 1 1 200px;
  min-width: 0;
}

.job-actions {
  align-self: flex-start;
}

/* 任务筛选工具栏的垂直分隔线固定高度，避免随 chip 换行而拉伸 */
.filter-divider {
  height: 24px;
  align-self: center;
}

/* 系统日志：等级标签定宽 + 信息列对齐，长文本仅在必要时断行 */
.log-level {
  min-width: 46px;
  justify-content: center;
}

.log-item + .log-item {
  border-top: 1px solid rgba(var(--v-border-color), 0.08);
}

.log-message {
  white-space: pre-wrap !important;
  overflow-wrap: anywhere !important;
  word-break: normal !important;
  line-height: 1.55 !important;
}

.log-meta {
  font-size: 11px !important;
  margin-top: 2px;
  overflow-wrap: anywhere;
}

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
