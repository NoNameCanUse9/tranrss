<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  mdiSync,
  mdiAlertCircleOutline,
  mdiPlus,
  mdiMagnify,
  mdiRssBox,
  mdiRss,
  mdiCheckCircleOutline,
  mdiTagOutline,
  mdiTranslate,
  mdiTextBoxSearchOutline,
  mdiNewspaper,
  mdiClockOutline,
  mdiRefresh,
  mdiPencilOutline,
  mdiTrashCanOutline,
  mdiClose,
  mdiLinkVariant,
  mdiEarth,
  mdiImageOutline,
  mdiFormatListText,
  mdiInformationOutline,
  mdiFolderOutline,
  mdiNumeric,
  mdiUpdate,
  mdiRobotOutline,
  mdiAutoFix,
  mdiAlertDecagram,
  mdiCogOutline,
  mdiShareVariant,
  mdiContentCopy,
  // mdiArchiveOutline,
  // mdiHeartOutline,
  // mdiHeart
} from '@mdi/js'
import { apiFetch } from '../../utils/api'

const { t } = useI18n()

interface Subscription {
  id: number
  feedId: number
  title: string
  url: string
  category: string
  articleCount: number
  lastSync: string | null
  status: 'active' | 'error'
  targetLanguage: string
  autoTranslate: boolean
  needSummary: boolean
  isShared: boolean
  siteUrl?: string | null
  description?: string | null
  iconUrl?: string | null
  iconBase64?: string | null
  num: number
  refreshInterval: number
  lastStatusCode?: number | null
  lastError?: string | null
}

const getHostname = (url: string) => {
  try {
    return new URL(url).hostname
  } catch (e) {
    return url
  }
}

const subscriptions = ref<Subscription[]>([])
const loading = ref(true)
const error = ref('')

const fetchSubscriptions = async () => {
  loading.value = true
  error.value = ''
  try {
    const response = await apiFetch('/api/feeds')
    if (!response.ok) throw new Error(t('sub.fetch_failed'))
    subscriptions.value = await response.json()
  } catch (e: any) {
    error.value = e?.message || t('sub.fetch_failed')
  } finally {
    loading.value = false
  }
}

onMounted(fetchSubscriptions)

const dialog = ref(false)
const deleteDialog = ref(false)
const selectedSub = ref<Subscription | null>(null)
const search = ref('')
const syncing = ref<Record<number, boolean>>({})
const saving = ref(false)
const fetchingPreview = ref(false)
const snackbar = ref({ show: false, text: '', color: 'success' })
const inactiveDialog = ref(false)
const inactiveFeeds = ref<any[]>([])
const inactiveSearch = ref('')
const selectedInactive = ref<number[]>([])
const activating = ref(false)
const isNonStandard = ref(false)
const isEditMode = computed(() => !!selectedSub.value)
const deletingIds = ref<Set<number>>(new Set())
let previewRequestId = 0

const filteredInactive = computed(() => {
  if (!inactiveSearch.value) return inactiveFeeds.value
  const q = inactiveSearch.value.toLowerCase()
  return inactiveFeeds.value.filter(
    f => f.title.toLowerCase().includes(q) || f.url.toLowerCase().includes(q)
  )
})

const fetchInactive = async (): Promise<boolean> => {
  try {
    const response = await apiFetch('/api/feeds/inactive')
    if (!response.ok) throw new Error(t('sub.inactive_fetch_failed'))
    inactiveFeeds.value = await response.json()
    return true
  } catch (e: any) {
    snackbar.value = { show: true, text: e?.message || t('sub.inactive_fetch_failed'), color: 'error' }
    return false
  }
}

const openInactiveDialog = async () => {
  selectedInactive.value = []
  inactiveSearch.value = ''
  if (!(await fetchInactive())) return
  inactiveDialog.value = true
}

const activateSelected = async () => {
  const available = new Set(inactiveFeeds.value.map(f => f.feed_id))
  const ids = selectedInactive.value.filter(id => available.has(id))
  if (ids.length === 0) return
  activating.value = true
  try {
    const response = await apiFetch('/api/feeds/inactive/activate', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ feed_ids: ids })
    })
    if (!response.ok) throw new Error(t('sub.restore_failed'))
    snackbar.value = { show: true, text: t('sub.restore_success', { n: ids.length }), color: 'success' }
    inactiveDialog.value = false
    selectedInactive.value = []
    fetchSubscriptions()
  } catch (e: any) {
    snackbar.value = { show: true, text: e.message || t('sub.restore_failed'), color: 'error' }
  } finally {
    activating.value = false
  }
}

const toggleSelectAllInactive = () => {
  if (selectedInactive.value.length === filteredInactive.value.length) {
    selectedInactive.value = []
  } else {
    selectedInactive.value = filteredInactive.value.map(f => f.feed_id)
  }
}

const categories = computed(() => {
  const existing = subscriptions.value
    .map(s => s.category)
    .filter(c => c && c !== '未分类')
  return [...new Set(existing)]
})

const form = ref({
  title: '',
  url: '',
  category: '',
  targetLanguage: 'Chinese',
  autoTranslate: false,
  needSummary: false,
  siteUrl: '',
  description: '',
  iconUrl: '',
  iconBase64: '',
  num: null as number | null,
  refreshInterval: 30,
  isShared: false,
})

const filtered = computed(() => {
  if (!search.value) return subscriptions.value
  const q = search.value.toLowerCase()
  return subscriptions.value.filter(
    s => s.title.toLowerCase().includes(q) || s.url.toLowerCase().includes(q) || s.category.toLowerCase().includes(q)
  )
})

const getSubStatus = (sub: Subscription) => {
  if (sub.lastError) return 'error'
  if (sub.lastStatusCode && (sub.lastStatusCode < 200 || sub.lastStatusCode >= 300)) return 'error'
  return 'active'
}

const statusInfo = (sub: Subscription): { color: string; label: string; icon: string } => {
  const status = getSubStatus(sub)
    if (status === 'error') {
      let label = t('sub.status_error')
      if (sub.lastStatusCode) {
        label = `${t('sub.status_error')} ${sub.lastStatusCode}`
      }
      return { color: 'error', label, icon: mdiAlertCircleOutline }
    }
    return { color: 'success', label: t('sub.status_normal'), icon: mdiCheckCircleOutline }
}

const syncNow = async (id: number) => {
  syncing.value[id] = true
  try {
    const response = await apiFetch(`/api/feeds/${id}/sync`, {
      method: 'POST'
    })
    if (!response.ok) throw new Error(t('common.sync_failed'))
    
    snackbar.value = { show: true, text: t('sub.sync_started'), color: 'success' }
    await fetchSubscriptions()
  } catch (e: any) {
    snackbar.value = { show: true, text: e.message || t('common.sync_failed'), color: 'error' }
  } finally {
    syncing.value[id] = false
  }
}

const syncAllLoading = ref(false)
const syncAll = async () => {
  if (subscriptions.value.length === 0) return
  syncAllLoading.value = true
  try {
    const response = await apiFetch('/api/feeds/sync_all', {
      method: 'POST'
    })
    if (!response.ok) throw new Error(t('common.sync_failed'))
    
    snackbar.value = { show: true, text: t('sub.sync_started'), color: 'success' }
    await fetchSubscriptions()
  } catch (e: any) {
    snackbar.value = { show: true, text: e.message || t('common.sync_failed'), color: 'error' }
  } finally {
    syncAllLoading.value = false
  }
}

const openAddDialog = () => {
  form.value = { 
    title: '', url: '', category: '', targetLanguage: 'Chinese', autoTranslate: false, needSummary: false,
    siteUrl: '', description: '', iconUrl: '', iconBase64: '', num: null, refreshInterval: 30, isShared: false
  }
  selectedSub.value = null
  isNonStandard.value = false
  dialog.value = true
}

const openEditDialog = (sub: Subscription) => {
  form.value = { 
    title: sub.title, 
    url: sub.url, 
    category: sub.category, 
    targetLanguage: sub.targetLanguage || 'Chinese',
    autoTranslate: sub.autoTranslate,
    needSummary: sub.needSummary,
    siteUrl: sub.siteUrl || '',
    description: sub.description || '',
    iconUrl: sub.iconUrl || '',
    iconBase64: sub.iconBase64 || '',
    num: sub.num,
    refreshInterval: sub.refreshInterval || 30,
    isShared: sub.isShared || false,
  }
  selectedSub.value = sub
  isNonStandard.value = false
  dialog.value = true
}

const confirmDelete = (sub: Subscription) => {
  selectedSub.value = sub
  deleteDialog.value = true
}

const deleteSub = async () => {
  const sub = selectedSub.value
  if (!sub || deletingIds.value.has(sub.id)) return

  const started = new Set(deletingIds.value)
  started.add(sub.id)
  deletingIds.value = started

  let succeeded = false
  try {
    const response = await apiFetch(`/api/feeds/${sub.id}`, {
      method: 'DELETE'
    })
    if (!response.ok) throw new Error(t('sub.delete_failed'))
    succeeded = true
    await fetchSubscriptions()
  } catch (e: any) {
    snackbar.value = { show: true, text: e?.message || t('sub.delete_failed'), color: 'error' }
  } finally {
    const remaining = new Set(deletingIds.value)
    remaining.delete(sub.id)
    deletingIds.value = remaining
  }
  if (succeeded) deleteDialog.value = false
}

const saveSub = async () => {
  saving.value = true
  try {
    const isEdit = !!selectedSub.value
    const url = isEdit ? `/api/feeds/${selectedSub.value!.id}` : '/api/feeds'
    const method = isEdit ? 'PUT' : 'POST'

    // 调整参数名以匹配后端模型；更新接口会忽略 feedUrl/元数据字段，仅在创建时提交
    const payload: Record<string, unknown> = {
      category: form.value.category,
      customTitle: form.value.title,
      needTranslate: form.value.autoTranslate,
      needSummary: form.value.needSummary,
      targetLanguage: form.value.targetLanguage,
      num: form.value.num ?? undefined,
      refreshInterval: form.value.refreshInterval,
      isShared: form.value.isShared,
      // folderId: ... // 暂时默认由后端根据 category 自动处理
    }

    if (!isEdit) {
      Object.assign(payload, {
        feedUrl: form.value.url,
        siteUrl: form.value.siteUrl,
        description: form.value.description,
        iconUrl: form.value.iconUrl,
        iconBase64: form.value.iconBase64,
      })
    }

    const response = await apiFetch(url, {
      method,
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload),
    })

    if (!response.ok) throw new Error(t('sub.save_failed'))

    snackbar.value = {
      show: true,
      text: isEdit ? t('sub.edit_success') : t('sub.add_success'),
      color: 'success'
    }
    await fetchSubscriptions()
    dialog.value = false
  } catch (e: any) {
    snackbar.value = { show: true, text: e?.message || t('sub.save_failed'), color: 'error' }
    console.error(e)
  } finally {
    saving.value = false
  }
}

const formatDate = (dateStr: string | null) => {
  if (!dateStr) return '-'
  const date = new Date(dateStr)
  return date.toLocaleString()
}

const handleUrlBlur = async () => {
  if (!form.value.url || selectedSub.value) return

  const requestId = ++previewRequestId
  const requestedUrl = form.value.url
  const isCurrent = () =>
    requestId === previewRequestId && dialog.value && !selectedSub.value && form.value.url === requestedUrl

  fetchingPreview.value = true
  isNonStandard.value = false
  try {
    const response = await apiFetch('/api/feeds/preview', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ url: requestedUrl })
    })

    if (!response.ok) throw new Error(t('sub.preview_failed'))

    const data = await response.json()
    if (!isCurrent()) return

    // 如果后端返回了非标标记
    if (data.title === 'Non-standard RSS Source') {
      isNonStandard.value = true
      snackbar.value = {
        show: true,
        text: t('sub.nonstandard_warning'),
        color: 'warning'
      }
    }

    if (data.title && !form.value.title) {
      form.value.title = data.title
    }
    if (!form.value.siteUrl) form.value.siteUrl = data.siteUrl || ''
    if (!form.value.description) form.value.description = data.description || ''
    if (!form.value.iconUrl) form.value.iconUrl = data.iconUrl || ''
    if (!form.value.iconBase64) form.value.iconBase64 = data.iconBase64 || ''
  } catch (e: any) {
    if (!isCurrent()) return
    snackbar.value = {
      show: true,
      text: e?.message || t('sub.preview_failed_hint'),
      color: 'error'
    }
  } finally {
    if (requestId === previewRequestId) fetchingPreview.value = false
  }
}

const shareUrl = computed(() => {
  const feedId = selectedSub.value?.feedId
  if (!feedId) return ''
  return `${window.location.origin}/share/${feedId}`
})

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text)
    snackbar.value = { show: true, text: t('common.copy_success'), color: 'success' }
  } catch (err) {
    snackbar.value = { show: true, text: t('sub.copy_failed'), color: 'error' }
  }
}

const copyShareUrl = () => {
  if (!shareUrl.value) return
  copyToClipboard(shareUrl.value)
}

watch([() => form.value.autoTranslate, () => form.value.needSummary], ([trans, summary]) => {
  if (!trans && !summary) {
    form.value.isShared = false
  }
})

</script>

<template>
  <div class="subscription-view">
    <div class="sticky-header pa-4 pb-0 mb-4">
      <div class="d-flex align-center justify-space-between flex-wrap gap-3 mb-6">
        <div class="page-header-title min-w-0">
          <h1 class="text-h3 font-weight-bold">{{ $t('sub.title') }}</h1>
          <p class="text-body-1 text-medium-emphasis mt-2">{{ $t('sub.active_count', { n: subscriptions.length }) }}</p>
        </div>
        <div class="page-header-actions d-flex flex-wrap gap-2">
          <v-btn 
            variant="tonal" 
            color="secondary" 
            rounded="pill" 
            class="text-none font-weight-bold" 
            @click="syncAll"
            :loading="syncAllLoading"
            :disabled="subscriptions.length === 0"
          >
            <v-icon start :icon="mdiSync" />
            {{ $t('sub.sync_all') }}
          </v-btn>
          <v-btn 
            color="warning" 
            variant="tonal"
            rounded="pill" 
            class="text-none font-weight-bold mr-2" 
            @click="openInactiveDialog"
          >
            <v-icon start :icon="mdiAlertCircleOutline" />
            {{ $t('sub.inactive_list') }}
          </v-btn>
          <v-btn color="primary" rounded="pill" elevation="0" class="text-none font-weight-bold" @click="openAddDialog">
            <v-icon start :icon="mdiPlus" />
            {{ $t('sub.add_btn') }}
          </v-btn>
        </div>
      </div>

      <!-- 搜索 -->
      <v-text-field
        v-model="search"
        :placeholder="$t('sub.search')"
        variant="outlined"
        density="comfortable"
        rounded="xl"
        class="mb-4"
        color="primary"
        :prepend-inner-icon="mdiMagnify"
        hide-details
        clearable
      />
    </div>

    <div class="pa-4 pt-0">

    <!-- 加载状态 -->
    <v-card v-if="loading" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-12">
      <v-progress-circular indeterminate color="primary" size="48" class="mb-4" />
      <p class="text-body-1">{{ $t('sub.loading') }}</p>
    </v-card>

    <!-- 加载失败 -->
    <v-card v-else-if="error" rounded="xl" variant="tonal" color="error" class="text-center pa-12">
      <v-icon size="64" color="error" class="mb-4">{{ mdiAlertCircleOutline }}</v-icon>
      <h3 class="text-h6 mb-2">{{ $t('sub.load_error_title') }}</h3>
      <p class="text-body-2 text-medium-emphasis mb-6">{{ error }}</p>
      <v-btn color="primary" rounded="pill" elevation="0" class="text-none" @click="fetchSubscriptions">
        <v-icon start :icon="mdiRefresh" />
        {{ $t('common.retry') }}
      </v-btn>
    </v-card>

    <!-- 空状态 -->
    <v-card v-else-if="filtered.length === 0" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-12">
      <v-icon size="64" color="primary" class="mb-4">{{ mdiRssBox }}</v-icon>
      <h3 class="text-h6 mb-2">{{ search ? $t('sub.empty_search') : $t('sub.empty_no_feeds') }}</h3>
      <p class="text-body-2 text-medium-emphasis mb-6">{{ search ? $t('sub.empty_try_other') : $t('sub.empty_add_first') }}</p>
      <v-btn v-if="!search" color="primary" rounded="pill" elevation="0" class="text-none" @click="openAddDialog">
        {{ $t('sub.add_first') }}
      </v-btn>
    </v-card>

    <v-row v-else>
      <v-col v-for="sub in filtered" :key="sub.id" cols="12" sm="6" md="6" lg="4" xl="3" class="d-flex sub-card-col">
        <v-card rounded="xl" variant="flat" color="surface" class="sub-card h-100 w-100">
          <v-card-text class="pa-5 d-flex flex-column h-100">
            <div class="d-flex align-start justify-space-between gap-2 mb-2">
              <div class="flex-1 min-w-0">
                <div class="d-flex align-center gap-2 mb-1 min-w-0">
                <v-avatar size="20" rounded="sm" v-if="sub.iconBase64 || sub.iconUrl" class="bg-grey-lighten-4 flex-shrink-0">
                  <v-img :src="sub.iconBase64 || sub.iconUrl">
                      <template v-slot:placeholder>
                        <v-icon size="14" color="grey">{{ mdiRss }}</v-icon>
                      </template>
                    </v-img>
                  </v-avatar>
                  <p class="text-body-1 font-weight-semibold text-truncate flex-grow-1 mb-0">{{ sub.title }}</p>
                </div>
                <p class="text-caption text-medium-emphasis text-truncate mb-0">{{ sub.url }}</p>
              </div>
              <v-tooltip v-if="sub.lastError" location="top" offset="10">
                <template v-slot:activator="{ props }">
                  <v-chip
                    v-bind="props"
                    :color="statusInfo(sub).color"
                    size="x-small"
                    variant="tonal"
                    class="text-none flex-shrink-0 cursor-help"
                  >
                    <v-icon start size="12" :icon="statusInfo(sub).icon" />
                    {{ statusInfo(sub).label }}
                  </v-chip>
                </template>
                <div class="text-caption pa-1" style="max-width: 300px">{{ sub.lastError }}</div>
              </v-tooltip>
              <v-chip
                v-else
                :color="statusInfo(sub).color"
                size="x-small"
                variant="tonal"
                class="text-none flex-shrink-0"
              >
                <v-icon start size="12" :icon="statusInfo(sub).icon" />
                {{ statusInfo(sub).label }}
              </v-chip>
            </div>

            <div class="d-flex flex-wrap align-center gap-2 mb-3">
              <v-chip size="x-small" variant="outlined" color="primary" class="text-none">
                <v-icon start size="12" :icon="mdiTagOutline" />
                {{ sub.category === '未分类' ? $t('common.uncategorized') : sub.category }}
              </v-chip>
              <v-chip v-if="sub.autoTranslate" size="x-small" variant="outlined" color="secondary" class="text-none">
                <v-icon start size="12" :icon="mdiTranslate" />
                {{ $t('sub.auto_translate') }}
              </v-chip>
              <v-chip v-if="sub.needSummary" size="x-small" variant="outlined" color="info" class="text-none">
                <v-icon start size="12" :icon="mdiTextBoxSearchOutline" />
                {{ $t('sub.summary') }}
              </v-chip>
            </div>

            <v-divider class="mb-3" />
            
            <div class="text-caption text-medium-emphasis line-clamp-desc mb-3">
              {{ sub.description || '' }}
            </div>

            <v-spacer />

            <div class="d-flex align-center gap-4 text-caption text-medium-emphasis mb-4 flex-wrap">
              <span class="d-flex align-center text-no-wrap flex-shrink-0"><v-icon size="14" class="mr-1" :icon="mdiNewspaper" />{{ $t('sub.article_count', { n: sub.articleCount }) }}</span>
              <span class="d-flex align-center min-w-0">
                <v-icon size="14" class="mr-1 flex-shrink-0" :icon="mdiClockOutline" />
                <span class="text-truncate">{{ formatDate(sub.lastSync) }}</span>
              </span>
            </div>

            <div class="d-flex align-center gap-2">
              <v-btn
                variant="tonal"
                color="primary"
                size="small"
                rounded="pill"
                class="text-none flex-1"
                :loading="syncing[sub.id]"
                @click="syncNow(sub.id)"
              >
                <v-icon start size="16" :icon="mdiRefresh" />
                {{ $t('sub.sync') }}
              </v-btn>
              <v-btn 
                variant="tonal" 
                color="secondary" 
                size="small" 
                rounded="circle" 
                class="text-none" 
                min-width="36"
                @click="openEditDialog(sub)"
              >
                <v-icon size="16" :icon="mdiPencilOutline" />
              </v-btn>
              <v-btn 
                variant="tonal" 
                color="error" 
                size="small" 
                rounded="circle" 
                class="text-none" 
                min-width="36"
                :loading="deletingIds.has(sub.id)"
                :disabled="deletingIds.has(sub.id)"
                @click="confirmDelete(sub)"
              >
                <v-icon size="16" :icon="mdiTrashCanOutline" />
              </v-btn>
            </div>
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>

    <!-- 添加/编辑对话框 -->
    <v-dialog v-model="dialog" width="60%" :scrim="true" persistent scrollable>
      <v-card rounded="xl" class="subscription-dialog shadow-premium">
        <div class="dialog-header pa-6 d-flex align-center justify-space-between">
          <div>
            <h2 class="text-h5 font-weight-bold gradient-text">
              {{ selectedSub ? $t('sub.dialog_edit_title') : $t('sub.dialog_add_title') }}
            </h2>
            <p class="text-caption text-medium-emphasis mt-1">
              {{ selectedSub ? $t('sub.dialog_edit_sub') : $t('sub.dialog_add_sub') }}
            </p>
          </div>
          <v-btn :icon="mdiClose" variant="text" color="error" rounded="pill" @click="dialog = false"></v-btn>
        </div>

        <v-divider />

        <v-card-text class="pa-8 custom-scrollbar" style="max-height: 70vh;">
          <div class="d-flex flex-column gap-8">
            <!-- 第 1 步：订阅源基本信息 -->
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="primary" class="mr-2">{{ mdiRss }}</v-icon>
                {{ $t('sub.metadata_section') }}
              </h3>
              
              <div class="d-flex flex-column gap-6 w-100">
                <!-- 实时预览卡片 -->
                <div class="preview-card pa-6 rounded-xl text-center d-flex flex-column align-center gap-2 mb-2 w-100">
                <v-avatar size="80" rounded="xl" class="elevation-1 mb-2" color="white">
                  <v-img :src="form.iconBase64 || form.iconUrl || ''" cover>
                      <template v-slot:placeholder>
                        <v-icon size="40" color="grey-lighten-2">{{ mdiRssBox }}</v-icon>
                      </template>
                    </v-img>
                  </v-avatar>
                  <div v-if="form.title || form.url" class="text-h6 font-weight-bold text-truncate">
                    {{ form.title || $t('sub.add_btn') }}
                  </div>
                  <div v-if="form.siteUrl" class="text-body-2 text-primary text-decoration-none d-flex align-center">
                    <v-icon size="14" class="mr-1">{{ mdiLinkVariant }}</v-icon>
                    {{ getHostname(form.siteUrl) }}
                  </div>
                </div>

                <v-alert
                  v-if="isNonStandard"
                  type="warning"
                  variant="tonal"
                  density="compact"
                  :icon="mdiAlertDecagram"
                  class="text-caption mb-2"
                >
                  {{ $t('sub.nonstandard_alert') }}
                </v-alert>

                <div class="d-flex flex-column gap-4 w-100">
                  <v-text-field 
                    v-model="form.url" 
                    :label="$t('sub.url')" 
                    variant="outlined" 
                    density="comfortable" 
                    rounded="lg" 
                    color="primary" 
                    :prepend-inner-icon="mdiRss" 
                    :hint="isEditMode ? $t('sub.metadata_readonly_hint') : $t('sub.url_hint')"
                    persistent-hint
                    :readonly="isEditMode"
                    @blur="handleUrlBlur"
                    :loading="fetchingPreview"
                    class="w-100"
                  />
                  
                  <div class="d-flex gap-4 w-100">
                    <v-text-field 
                      v-model="form.siteUrl" 
                      :label="$t('sub.site_url')" 
                      variant="outlined" 
                      density="comfortable" 
                      rounded="lg" 
                      color="primary" 
                      :prepend-inner-icon="mdiEarth" 
                      :readonly="isEditMode"
                      hide-details 
                      style="flex: 1"
                    />
                    <v-text-field 
                      v-model="form.iconUrl" 
                      :label="$t('sub.icon_url')" 
                      variant="outlined" 
                      density="comfortable" 
                      rounded="lg" 
                      color="primary" 
                      :prepend-inner-icon="mdiImageOutline" 
                      :readonly="isEditMode"
                      hide-details 
                      style="flex: 1"
                    />
                  </div>

                  <v-textarea 
                    v-model="form.description" 
                    :label="$t('sub.desc')" 
                    variant="outlined" 
                    density="comfortable" 
                    rounded="lg" 
                    :prepend-inner-icon="mdiFormatListText" 
                    :readonly="isEditMode"
                    hide-details 
                    rows="2"
                    class="w-100"
                  />
                </div>
              </div>

              <v-alert
                v-if="!form.url"
                type="info"
                variant="tonal"
                density="compact"
                :icon="mdiInformationOutline"
                class="text-caption mt-6"
              >
                {{ $t('sub.preview_hint') }}
              </v-alert>
            </section>

            <v-divider />

            <!-- 第 2 步：个人订阅配置 -->
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="secondary" class="mr-2">{{ mdiCogOutline }}</v-icon>
                {{ $t('settings.preferences') }}
              </h3>
              <div class="d-flex flex-column gap-4 w-100">
                <div class="d-flex gap-4 w-100 mb-2">
                  <v-text-field 
                    v-model="form.title" 
                    :label="$t('sub.custom_title')" 
                    variant="outlined" 
                    density="comfortable" 
                    rounded="lg" 
                    color="primary" 
                    :prepend-inner-icon="mdiPencilOutline" 
                    :hint="$t('sub.custom_title_hint')"
                    persistent-hint
                    style="flex: 5"
                  />
                  <v-combobox 
                    v-model="form.category" 
                    :items="categories" 
                    :label="$t('sub.category')" 
                    variant="outlined" 
                    density="comfortable" 
                    rounded="lg" 
                    color="primary" 
                    :prepend-inner-icon="mdiFolderOutline" 
                    persistent-hint
                    hint=""
                    hide-no-data
                    style="flex: 3"
                  />
                  <v-text-field 
                    v-model.number="form.num" 
                    :label="$t('sub.max_storage')" 
                    type="number"
                    variant="outlined" 
                    density="comfortable" 
                    rounded="lg" 
                    color="primary" 
                    :prepend-inner-icon="mdiNumeric" 
                    persistent-hint
                    :hint="$t('sub.max_storage_hint')"
                    style="flex: 2"
                  />
                  <v-text-field 
                    v-model.number="form.refreshInterval" 
                    :label="$t('sub.refresh_interval')" 
                    type="number"
                    variant="outlined" 
                    density="comfortable" 
                    rounded="lg" 
                    color="primary" 
                    :prepend-inner-icon="mdiUpdate" 
                    persistent-hint
                    :hint="$t('sub.refresh_interval_hint')"
                    style="flex: 2"
                  />
                </div>
              </div>
            </section>

            <v-divider />

            <!-- 第 3 步：AI 智能增强 -->
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="info" class="mr-2">{{ mdiRobotOutline }}</v-icon>
                {{ $t('sub.ai_enhance') }}
              </h3>
              <v-card variant="tonal" border color="primary" rounded="lg" class="pa-4">
                <div class="d-flex align-center justify-space-between mb-4">
                  <div class="d-flex align-center">
                    <v-avatar color="primary" size="32" class="mr-3">
                      <v-icon size="18" color="white">{{ mdiTranslate }}</v-icon>
                    </v-avatar>
                    <div>
                      <p class="text-body-2 font-weight-bold">{{ $t('sub.auto_translate') }}</p>
                      <p class="text-caption text-medium-emphasis">{{ $t('sub.auto_translate_desc') }}</p>
                    </div>
                  </div>
                  <v-switch v-model="form.autoTranslate" color="primary" hide-details density="compact" />
                </div>
                
                <v-divider class="my-3 opacity-20" />

                <div class="d-flex align-center justify-space-between">
                  <div class="d-flex align-center">
                    <v-avatar color="secondary" size="32" class="mr-3">
                      <v-icon size="18" color="white">{{ mdiAutoFix }}</v-icon>
                    </v-avatar>
                    <div>
                      <p class="text-body-2 font-weight-bold">{{ $t('sub.summary') }}</p>
                      <p class="text-caption text-medium-emphasis">{{ $t('sub.summary_desc') }}</p>
                    </div>
                  </div>
                  <v-switch v-model="form.needSummary" color="secondary" hide-details density="compact" />
                </div>

                <v-divider class="my-3 opacity-20" />

                <div class="d-flex align-center justify-space-between">
                  <div class="d-flex align-center">
                    <v-avatar color="info" size="32" class="mr-3">
                      <v-icon size="18" color="white">{{ mdiShareVariant }}</v-icon>
                    </v-avatar>
                    <div>
                      <p class="text-body-2 font-weight-bold">{{ $t('sub.share_title') }}</p>
                      <p class="text-caption text-medium-emphasis">{{ $t('sub.share_desc') }}</p>
                    </div>
                  </div>
                  <v-switch 
                    v-model="form.isShared" 
                    color="info" 
                    hide-details 
                    density="compact"
                    :disabled="!form.autoTranslate && !form.needSummary"
                  />
                </div>

                <v-expand-transition>
                  <div v-if="form.isShared" class="mt-4 pa-3 bg-surface rounded-lg border-thin d-flex align-center gap-2">
                    <code class="text-caption text-truncate flex-1">{{ shareUrl }}</code>
                    <v-btn size="x-small" variant="tonal" color="primary" :disabled="!shareUrl" @click="copyShareUrl">
                      <v-icon start size="14">{{ mdiContentCopy }}</v-icon>
                      {{ $t('common.copy') }}
                    </v-btn>
                  </div>
                </v-expand-transition>

                <!-- 目标语言选择：仅在开启 AI 功能时显示 -->
                <v-expand-transition>
                  <div v-if="form.autoTranslate || form.needSummary">
                    <v-divider class="my-4" />
                    <div class="d-flex align-center gap-4">
                      <v-icon color="primary" size="20">{{ mdiTranslate }}</v-icon>
                      <v-select 
                        v-model="form.targetLanguage" 
                        :items="[
                          { title: t('sub.lang_zh'), value: 'Chinese' },
                          { title: t('sub.lang_en'), value: 'English' },
                          { title: t('sub.lang_ja'), value: 'Japanese' },
                          { title: t('sub.lang_fr'), value: 'French' },
                          { title: t('sub.lang_de'), value: 'German' },
                          { title: t('sub.lang_es'), value: 'Spanish' },
                          { title: t('sub.lang_ru'), value: 'Russian' },
                          { title: t('sub.lang_ko'), value: 'Korean' }
                        ]" 
                        :label="$t('sub.target_lang')" 
                        variant="outlined" 
                        density="compact" 
                        rounded="lg" 
                        color="primary" 
                        hide-details 
                        class="flex-1"
                      />
                    </div>
                  </div>
                </v-expand-transition>
              </v-card>
            </section>
          </div>
        </v-card-text>

        <v-divider />

        <v-card-actions class="pa-6">
          <v-spacer />
          
          <v-btn 
            color="primary" 
            class="text-none font-weight-bold px-10 btn-premium" 
            rounded="pill" 
            elevation="4" 
            :loading="saving" 
            @click="saveSub"
          >
            {{ selectedSub ? $t('sub.save') : $t('sub.subscribe_now') }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 删除确认 -->
    <v-dialog v-model="deleteDialog" max-width="360">
      <v-card rounded="xl">
        <v-card-title class="pa-6 pb-2 text-body-1 font-weight-bold">{{ $t('sub.confirm_delete') }}</v-card-title>
        <v-card-text class="pa-6 pt-2 text-body-2 text-medium-emphasis">
          {{ $t('sub.delete_msg', { name: selectedSub?.title }) }}
        </v-card-text>
        <v-card-actions class="pa-6 pt-0">
          <v-spacer />
          <v-btn variant="text" class="text-none" rounded="pill" :disabled="deletingIds.has(selectedSub?.id ?? -1)" @click="deleteDialog = false">{{ $t('common.cancel') }}</v-btn>
          <v-btn color="error" class="text-none font-weight-bold px-6" rounded="pill" elevation="0" :loading="deletingIds.has(selectedSub?.id ?? -1)" @click="deleteSub">{{ $t('common.delete') }}</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
    <!-- 提示消息 -->
    <v-snackbar v-model="snackbar.show" :color="snackbar.color" rounded="pill" elevation="12">
      {{ snackbar.text }}
      <template v-slot:actions>
        <v-btn variant="text" @click="snackbar.show = false">{{ $t('common.confirm') }}</v-btn>
      </template>
    </v-snackbar>
  </div>
    <!-- 失效列表弹窗 -->
    <v-dialog v-model="inactiveDialog" max-width="700" scrollable>
      <v-card rounded="xl" class="shadow-premium">
        <v-card-title class="pa-6 d-flex align-center bg-warning text-white">
          <v-icon start class="mr-2">{{ mdiAlertDecagram }}</v-icon>
          {{ $t('sub.inactive_subtitle') }}
          <v-spacer />
          <v-btn :icon="mdiClose" variant="text" density="comfortable" @click="inactiveDialog = false" color="white"></v-btn>
        </v-card-title>

        <v-card-text class="pa-6">
          <v-alert
            type="info"
            variant="tonal"
            class="mb-4"
            density="compact"
            :text="$t('sub.inactive_desc')"
          />

          <v-text-field
            v-model="inactiveSearch"
            :placeholder="$t('sub.inactive_search')"
            variant="solo-filled"
            density="comfortable"
            rounded="lg"
            flat
            :prepend-inner-icon="mdiMagnify"
            class="mb-4"
            hide-details
          />

          <div class="d-flex align-center mb-2 px-1">
            <v-checkbox
              :model-value="selectedInactive.length === filteredInactive.length && filteredInactive.length > 0"
              :indeterminate="selectedInactive.length > 0 && selectedInactive.length < filteredInactive.length"
              :label="$t('sub.select_all')"
              hide-details
              density="compact"
              @click="toggleSelectAllInactive"
            />
            <v-spacer />
            <span class="text-caption text-medium-emphasis">{{ $t('sub.selected_info', { n: selectedInactive.length, total: filteredInactive.length }) }}</span>
          </div>

          <v-divider />

          <v-list class="bg-transparent" v-if="filteredInactive.length > 0">
            <v-list-item v-for="item in filteredInactive" :key="item.feed_id" class="px-0 py-2 border-b">
              <template v-slot:prepend>
                <v-checkbox
                  v-model="selectedInactive"
                  :value="item.feed_id"
                  hide-details
                  density="compact"
                />
              </template>
              
              <v-list-item-title class="font-weight-bold">{{ item.title }}</v-list-item-title>
              <v-list-item-subtitle class="text-truncate">{{ item.url }}</v-list-item-subtitle>
              <v-list-item-subtitle v-if="item.reason" class="text-error mt-1 text-wrap text-caption">
                {{ $t('sub.reason', { r: item.reason }) }}
              </v-list-item-subtitle>
              
              <template v-slot:append>
                <span class="text-caption text-medium-emphasis">{{ new Date(item.disabled_at).toLocaleString() }}</span>
              </template>
            </v-list-item>
          </v-list>
          
          <div v-else class="text-center py-12 text-medium-emphasis">
            <v-icon size="48" color="grey-lighten-1" class="mb-2">{{ inactiveFeeds.length > 0 && inactiveSearch ? mdiMagnify : mdiCheckCircleOutline }}</v-icon>
            <p>{{ inactiveFeeds.length > 0 && inactiveSearch ? $t('sub.inactive_no_matches') : $t('sub.no_inactive') }}</p>
          </div>
        </v-card-text>

        <v-divider />
        
        <v-card-actions class="pa-4 bg-grey-lighten-4">
          <v-spacer />
          <v-btn 
            variant="text" 
            class="text-none" 
            @click="inactiveDialog = false"
            rounded="pill"
          >{{ $t('common.cancel') }}</v-btn>
          <v-btn 
            color="warning" 
            variant="flat"
            rounded="pill"
            class="px-6 text-none font-weight-bold"
            :disabled="selectedInactive.length === 0"
            :loading="activating"
            @click="activateSelected"
          >
            {{ $t('sub.reactivate') }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.subscription-view {
  --view-pad: 16px;
  min-width: 0;
}

/* 与外层 v-container 的 pa-4 / pa-sm-6 / pa-md-8 保持一致 */
@media (min-width: 600px) {
  .subscription-view { --view-pad: 24px; }
}

@media (min-width: 960px) {
  .subscription-view { --view-pad: 32px; }
}
/* 订阅数量没有后端上限。卡片高度按视口内实测（auto）缓存，离屏卡片跳过
   布局与绘制，滚动时只重排视口附近的内容。 */
.sub-card-col {
  content-visibility: auto;
  contain-intrinsic-size: auto 240px;
}


.page-header-title {
  flex: 1 1 260px;
  min-width: 0;
}

.page-header-actions {
  flex: 0 0 auto;
}

/* 超长标签（如分类名）在 chip 内部以省略号截断，而不是硬裁切。
   此处内容区仅含纯文本（图标在 __prepend），block 化不会影响垂直居中。 */
.sub-card :deep(.v-chip__content) {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sub-card {
  display: flex;
  flex-direction: column;
  transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.2s ease;
  overflow: hidden;
  border: 1px solid rgba(var(--v-theme-primary), 0.05) !important;
}
.sub-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 12px 30px rgba(var(--v-theme-primary), 0.1) !important;
  z-index: 10;
}
.line-clamp-desc {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  overflow: hidden;
  line-height: 1.6;
  min-height: 3.2em;
}

/* Premium Dialog Styles */
.subscription-dialog {
  display: flex;
  flex-direction: column;
  max-height: 90vh !important;
}
.custom-scrollbar {
  overflow-y: auto;
}
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(var(--v-theme-primary), 0.3);
}
.gradient-text {
  background: linear-gradient(135deg, var(--v-theme-primary) 0%, #2c3e50 100%);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}
.shadow-premium {
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25) !important;
}
.border-e {
  border-right: 1px solid rgba(0, 0, 0, 0.05);
}
.preview-card {
  background: white;
  border: 1px solid rgba(0, 0, 0, 0.05);
  transition: transform 0.3s ease;
}
.preview-card:hover {
  transform: translateY(-2px);
}
.btn-premium {
  letter-spacing: 0.5px;
  text-transform: none;
  background: linear-gradient(135deg, var(--v-theme-primary) 0%, var(--v-theme-secondary) 100%) !important;
}

/* Sticky Header Styles */
.sticky-header {
  position: sticky;
  top: 0;
  z-index: 100;
  background: rgba(var(--v-theme-background), 0.85); /* 稍微透明背景 */
  backdrop-filter: blur(10px); /* 磨砂效果 */
  margin-left: calc(var(--view-pad) * -1); /* 背景延伸到容器内边距，避免横向溢出 */
  margin-right: calc(var(--view-pad) * -1);
  /* +16px 与卡片区外层 .pa-4 对齐，保证页头与卡片左边缘齐平 */
  padding-left: calc(var(--view-pad) + 16px) !important;
  padding-right: calc(var(--view-pad) + 16px) !important;
  border-bottom: 1px solid rgba(var(--v-theme-primary), 0.05);
}
</style>
