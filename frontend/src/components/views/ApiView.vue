<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  mdiBrain,
  mdiTranslate,
  mdiKeyOutline,
  mdiPlus,
  mdiShieldKeyOutline,
  mdiEyeOffOutline,
  mdiEyeOutline,
  mdiInformationOutline,
  mdiPencilOutline,
  mdiTrashCanOutline,
  mdiClose,
  mdiKeyVariant,
  mdiLabelOutline,
  mdiApps,
  mdiLinkVariant,
  mdiRefresh,
  mdiFormatTextWrappingOverflow,
  mdiSpeedometer,
  mdiCogOutline,
  mdiClockOutline,
  mdiTimerOutline,
  mdiChartDonut,
  mdiChartTimelineVariant,
  mdiPoll
} from '@mdi/js'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { PieChart, BarChart, LineChart } from 'echarts/charts'
import {
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent
} from 'echarts/components'
import VChart from 'vue-echarts'
import { computed } from 'vue'
import { apiFetch } from '../../utils/api'

use([
  CanvasRenderer,
  PieChart,
  BarChart,
  LineChart,
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent
])

const { t } = useI18n()

interface ApiConfig {
  id: number
  name: string
  api_type: string
  api_key: string | null
  base_url: string | null
  settings: string | Record<string, any>
  timeout_seconds: number
  retry_count: number
  retry_interval_ms: number
  retry_enabled: boolean
}

const apiConfigs = ref<ApiConfig[]>([])
const userSettings = ref<any>(null)
const loading = ref(false)
const statsLoading = ref(false)
const saving = ref(false)

interface UsageStats {
  total_prompt_tokens: number
  total_completion_tokens: number
  total_tokens: number
  usage_by_model: {
    model: string
    prompt_tokens: number
    completion_tokens: number
    total_tokens: number
  }[]
}

interface TimeSeriesUsage {
  date: string
  api_config_id: number
  model: string
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
}

const usageStats = ref<UsageStats | null>(null)
const usageHistory = ref<TimeSeriesUsage[]>([])
const selectedFilterApi = ref<number | null>(null)
const selectedFilterModel = ref<string>('全部模型')
const selectedRange = ref('all')

const dialog = ref(false)
const deleteDialog = ref(false)
const selectedConfig = ref<ApiConfig | null>(null)
const showToken = ref<Record<number, boolean>>({})
const detailDialog = ref(false)
const detailConfig = ref<ApiConfig | null>(null)
const statsDialog = ref(false)

const openDetail = (config: ApiConfig) => {
  detailConfig.value = config
  detailDialog.value = true
}

// 默认提供商配置，用于 UI 展示（图标、颜色等）
const providers = [
  { title: 'OpenAI', value: 'openai', icon: mdiBrain, color: '#10a37f' },
  { title: 'DeepLX', value: 'deeplx', icon: mdiTranslate, color: '#0f2b46' },
]

// 初始表单状态
const form = ref({
  name: '',
  api_type: 'openai',
  api_key: '',
  base_url: '',
  settings: { model: 'gpt-4o-mini', max_tokens: 20480, rpm: 3 } as Record<string, any>,
  timeout_seconds: 180,
  retry_count: 3,
  retry_interval_ms: 1000,
  retry_enabled: true,
})

// 从后端拉取配置
const fetchConfigs = async () => {
  loading.value = true
  try {
    const res = await apiFetch('/api/translate-configs')
    if (res.ok) {
      const data = await res.json()
      apiConfigs.value = data.map((item: any) => ({
        ...item,
        // 将数据库存的 JSON 字符串转回对象，方便前端操作
        settings: typeof item.settings === 'string' ? JSON.parse(item.settings) : item.settings
      }))
    }
  } catch (e) {
    console.error('获取 API 配置失败:', e)
  } finally {
    loading.value = false
  }
}

// 获取用户设置（用于判断哪些 API 是当前选中的）
const fetchUserSettings = async () => {
  try {
    const res = await apiFetch('/api/user/setting')
    if (res.ok) {
      userSettings.value = await res.json()
    }
  } catch (e) {
    console.error('获取用户设置失败:', e)
  }
}

const fetchUsageStats = async () => {
  statsLoading.value = true
  try {
    const res = await apiFetch('/api/translate-configs/usage')
    if (res.ok) {
      usageStats.value = await res.json()
    }
  } catch (e) {
    console.error('获取使用统计失败:', e)
  } finally {
    statsLoading.value = false
  }
}

const fetchUsageHistory = async () => {
  try {
    const res = await apiFetch('/api/translate-configs/usage/history')
    if (res.ok) {
      usageHistory.value = await res.json()
    }
  } catch (e) {
    console.error('获取使用历史失败:', e)
  }
}

onMounted(() => {
  fetchConfigs()
  fetchUserSettings()
  fetchUsageStats()
  fetchUsageHistory()
})

// 获取提供商的 UI 信息
const getProviderInfo = (type: string) =>
  providers.find(p => p.value === type) ?? { title: type, icon: mdiKeyOutline, color: 'primary' }

// 脱敏显示 Token
const maskToken = (token: string | null) => {
  if (!token) return t('api.not_set')
  return token.length > 8 ? token.slice(0, 4) + '••••••••' + token.slice(-4) : '••••••••'
}

// 打开添加弹窗
const openAddDialog = () => {
  form.value = { 
    name: '', 
    api_type: 'openai', 
    api_key: '', 
    base_url: '', 
    settings: { model: 'gpt-4o-mini', max_tokens: 20480, rpm: 3 },
    timeout_seconds: 180,
    retry_count: 3,
    retry_interval_ms: 1000,
    retry_enabled: true,
  }
  selectedConfig.value = null
  dialog.value = true
}

// 打开编辑弹窗
const openEditDialog = (config: ApiConfig) => {
  form.value = { 
    name: config.name, 
    api_type: config.api_type, 
    api_key: config.api_key || '', 
    base_url: config.base_url || '', 
    settings: { 
      model: (config.settings as any).model || 'gpt-4o-mini',
      max_tokens: (config.settings as any).max_tokens || 20480,
      rpm: (config.settings as any).rpm || 3 
    },
    timeout_seconds: config.timeout_seconds,
    retry_count: config.retry_count,
    retry_interval_ms: config.retry_interval_ms,
    retry_enabled: config.retry_enabled,
  }
  selectedConfig.value = config
  dialog.value = true
}

// 保存逻辑
const saveConfig = async () => {
  saving.value = true
  try {
    const isEdit = !!selectedConfig.value
    const url = isEdit ? `/api/translate-configs/${selectedConfig.value!.id}` : '/api/translate-configs'
    const method = isEdit ? 'PUT' : 'POST'
    
    const res = await apiFetch(url, {
      method,
      headers: { 
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        ...form.value,
        api_key: form.value.api_key || null,
        base_url: form.value.base_url || null,
      })
    })

    if (res.ok) {
      await fetchConfigs()
      dialog.value = false
    }
  } catch (e) {
    console.error('保存失败:', e)
  } finally {
    saving.value = false
  }
}

// 删除逻辑
const deleteConfig = async () => {
  if (!selectedConfig.value) return
  try {
    const res = await apiFetch(`/api/translate-configs/${selectedConfig.value.id}`, {
      method: 'DELETE'
    })
    if (res.ok) {
      apiConfigs.value = apiConfigs.value.filter(c => c.id !== selectedConfig.value!.id)
      deleteDialog.value = false
    }
  } catch (e) {
    console.error('删除失败:', e)
  }
}
const fetchingModels = ref(false)
const availableModels = ref<string[]>([])

const fetchModels = async () => {
  if (!form.value.base_url) return
  
  fetchingModels.value = true
  try {
    const url = form.value.base_url.replace(/\/$/, '') + '/models'
    const headers: Record<string, string> = {}
    if (form.value.api_key) {
      headers['Authorization'] = `Bearer ${form.value.api_key}`
    }

    const res = await fetch(url, { headers })
    if (res.ok) {
      const data = await res.json()
      if (data.data) {
        availableModels.value = data.data.map((m: any) => m.id)
      }
    }
  } catch (e) {
    console.error('获取模型列表失败:', e)
  } finally {
    fetchingModels.value = false
  }
}

const availableModelsList = computed(() => {
  const models = new Set(usageHistory.value.map(h => h.model))
  return ['全部模型', ...Array.from(models)]
})

const lineOption = computed(() => {
  if (!usageHistory.value.length) return {}
  
  let filtered = [...usageHistory.value]
  if (selectedFilterApi.value !== null) {
    filtered = filtered.filter(h => h.api_config_id === selectedFilterApi.value)
  }
  if (selectedFilterModel.value !== '全部模型') {
    filtered = filtered.filter(h => h.model === selectedFilterModel.value)
  }
  
  // Simple range filtering based on last N days for simplicity
  if (selectedRange.value !== 'all') {
    const now = new Date()
    let days = 7
    if (selectedRange.value === 'month') days = 30
    if (selectedRange.value === 'year') days = 365
    
    const cutoffDate = new Date(now.getTime() - days * 24 * 60 * 60 * 1000)
    const cutoffStr = cutoffDate.toISOString().split('T')[0] || ''
    filtered = filtered.filter(h => h.date >= cutoffStr)
  }
  
  const groupedByDate: Record<string, number> = {}
  filtered.forEach(h => {
    groupedByDate[h.date] = (groupedByDate[h.date] || 0) + h.total_tokens
  })
  
  const sortedDates = Object.keys(groupedByDate).sort()
  const data = sortedDates.map(d => groupedByDate[d])
  
  return {
    tooltip: { trigger: 'axis' },
    xAxis: { type: 'category', data: sortedDates, axisLabel: { rotate: sortedDates.length > 10 ? 45 : 0 } },
    yAxis: { type: 'value' },
    series: [
      {
        data: data,
        type: 'line',
        smooth: true,
        areaStyle: {
          color: {
            type: 'linear', x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(16, 163, 127, 0.3)' },
              { offset: 1, color: 'rgba(16, 163, 127, 0)' }
            ]
          }
        },
        itemStyle: { color: '#10a37f' },
        lineStyle: { width: 3 }
      }
    ]
  }
})

const pieOption = computed(() => {
  if (!usageStats.value) return {}
  return {
    tooltip: { trigger: 'item', formatter: '{b}: {c} ({d}%)' },
    legend: { bottom: '0', left: 'center', icon: 'circle', textStyle: { fontSize: 10 } },
    series: [
      {
        name: 'Model Usage',
        type: 'pie',
        radius: ['45%', '65%'],
        avoidLabelOverlap: false,
        itemStyle: { borderRadius: 8, borderColor: '#fff', borderWidth: 2 },
        label: { show: false },
        emphasis: { label: { show: true, fontSize: '14', fontWeight: 'bold' } },
        data: usageStats.value.usage_by_model.map(item => ({
          value: item.total_tokens,
          name: item.model
        }))
      }
    ]
  }
})
</script>

<template>
  <div class="api-view">
    <div class="d-flex align-center justify-space-between mb-6">
      <div>
        <h2 class="text-h5 font-weight-bold">{{ $t('api.title') }}</h2>
        <p class="text-body-2 text-medium-emphasis mt-1">{{ $t('api.subtitle') }}</p>
      </div>
      <div class="d-flex align-center gap-3">
        <!-- Token Detailed Stats Button -->
        <v-btn
          v-if="usageStats"
          variant="tonal"
          color="primary"
          rounded="pill"
          class="text-none font-weight-bold"
          @click="statsDialog = true"
        >
          <v-icon start>{{ mdiPoll }}</v-icon>
          Token 详情统计
        </v-btn>

        <v-btn
          color="primary"
          rounded="pill"
          elevation="0"
          class="text-none font-weight-bold"
          @click="openAddDialog"
        >
          <v-icon start>{{ mdiPlus }}</v-icon>
          {{ $t('api.add_btn') }}
        </v-btn>
      </div>
    </div>

    <!-- Token Usage Stats Section -->
    <v-row v-if="usageStats && usageStats.total_tokens > 0" id="token-usage-section" class="mb-8">
      <v-col cols="12">
        <v-card rounded="xl" variant="flat" color="surface-variant" class="pa-6 border-thin shadow-premium">
          <div class="d-flex align-center gap-3 mb-6">
            <v-avatar color="primary" variant="tonal" size="48">
              <v-icon color="primary">{{ mdiChartDonut }}</v-icon>
            </v-avatar>
            <div>
              <h3 class="text-h6 font-weight-bold">{{ $t('api.usage_title') }}</h3>
              <p class="text-caption text-medium-emphasis">基于 OpenAI 类型接口的 Tokens 实时统计</p>
            </div>
          </div>
          
          <v-row>
            <v-col cols="12" sm="4">
              <div class="pa-4 rounded-xl bg-surface border-thin text-center h-100 d-flex flex-column justify-center">
                <p class="text-caption text-medium-emphasis mb-1 font-weight-medium">{{ $t('api.total_tokens') }}</p>
                <p class="text-h4 font-weight-black text-primary">{{ usageStats.total_tokens.toLocaleString() }}</p>
              </div>
            </v-col>
            <v-col cols="12" sm="4">
              <div class="pa-4 rounded-xl bg-surface border-thin text-center h-100 d-flex flex-column justify-center">
                <p class="text-caption text-medium-emphasis mb-1 font-weight-medium">{{ $t('api.prompt_tokens_label') }}</p>
                <p class="text-h4 font-weight-black">{{ usageStats.total_prompt_tokens.toLocaleString() }}</p>
              </div>
            </v-col>
            <v-col cols="12" sm="4">
              <div class="pa-4 rounded-xl bg-surface border-thin text-center h-100 d-flex flex-column justify-center">
                <p class="text-caption text-medium-emphasis mb-1 font-weight-medium">{{ $t('api.completion_tokens_label') }}</p>
                <p class="text-h4 font-weight-black">{{ usageStats.total_completion_tokens.toLocaleString() }}</p>
              </div>
            </v-col>
          </v-row>

          <div class="mt-8">
            <p class="text-subtitle-2 font-weight-bold mb-4 d-flex align-center">
              <v-icon size="18" class="mr-2">{{ mdiChartTimelineVariant }}</v-icon>
              {{ $t('api.usage_by_model') }}
            </p>
            <div class="model-usage-list rounded-xl overflow-hidden border-thin bg-surface">
              <v-list class="pa-0">
                <v-list-item v-for="(item, index) in usageStats.usage_by_model" :key="item.model" :class="{'border-b-thin': index !== usageStats.usage_by_model.length - 1}" class="py-4 px-6">
                  <template v-slot:prepend>
                    <v-avatar color="primary" variant="tonal" size="36" class="mr-4">
                      <v-icon size="20">{{ mdiBrain }}</v-icon>
                    </v-avatar>
                  </template>
                  <v-list-item-title class="font-weight-bold text-subtitle-1">{{ item.model }}</v-list-item-title>
                  <v-list-item-subtitle class="mt-1">
                    <v-progress-linear
                      :model-value="(item.total_tokens / usageStats.total_tokens * 100)"
                      color="primary"
                      height="6"
                      rounded
                      class="mt-1"
                      style="width: 150px"
                    ></v-progress-linear>
                  </v-list-item-subtitle>
                  <template v-slot:append>
                    <div class="text-right">
                      <p class="text-h6 font-weight-bold mb-0">{{ item.total_tokens.toLocaleString() }} <span class="text-caption font-weight-medium text-medium-emphasis">Tokens</span></p>
                      <p class="text-caption text-medium-emphasis">
                        {{ item.prompt_tokens.toLocaleString() }} (In) / {{ item.completion_tokens.toLocaleString() }} (Out)
                      </p>
                    </div>
                  </template>
                </v-list-item>
              </v-list>
            </div>
          </div>
        </v-card>
      </v-col>
    </v-row>

    <!-- 密钥列表 -->
    <v-card v-if="apiConfigs.length === 0" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-12">
      <v-icon size="64" color="primary" class="mb-4">{{ mdiKeyOutline }}</v-icon>
      <h3 class="text-h6 mb-2">{{ $t('api.empty_title') }}</h3>
      <p class="text-body-2 text-medium-emphasis mb-6">{{ $t('api.empty_subtitle') }}</p>
      <v-btn color="primary" rounded="pill" elevation="0" class="text-none" @click="openAddDialog">
        {{ $t('api.add_first') }}
      </v-btn>
    </v-card>

    <!-- 密钥列表 -->
    <v-row v-else align="stretch">
      <v-col v-for="config in apiConfigs" :key="config.id" cols="12" md="6" class="d-flex">
        <v-card rounded="xl" variant="flat" color="surface-variant" class="api-key-card flex-grow-1">
          <v-card-text class="pa-5 d-flex flex-column">
            <!-- Header Section with Fixed Layout for Alignment -->
            <div class="d-flex align-start justify-space-between" style="min-height: 80px;">
              <div class="d-flex align-center gap-3">
                <v-avatar :color="getProviderInfo(config.api_type).color" size="40" rounded="lg">
                  <v-icon color="white" size="20">{{ getProviderInfo(config.api_type).icon }}</v-icon>
                </v-avatar>
                <div>
                  <p class="text-body-1 font-weight-semibold">{{ config.name }}</p>
                  <p class="text-caption text-medium-emphasis">{{ getProviderInfo(config.api_type).title }}</p>
                </div>
              </div>
              <div class="d-flex flex-wrap justify-end gap-1" style="max-width: 150px;">
                <v-chip color="success" size="x-small" variant="tonal" class="text-none">{{ $t('api.active') }}</v-chip>
                <v-chip v-if="userSettings?.translate_api_id === config.id" color="info" size="x-small" variant="tonal" class="text-none">{{ $t('api.translate') }}</v-chip>
                <v-chip v-if="userSettings?.summary_api_id === config.id" color="secondary" size="x-small" variant="tonal" class="text-none">{{ $t('api.summary') }}</v-chip>
              </div>
            </div>

            <v-divider class="my-3" />

            <!-- Middle Section: Token -->
            <div class="token-row d-flex align-center gap-2 mb-4">
              <v-icon size="16" color="medium-emphasis">{{ mdiShieldKeyOutline }}</v-icon>
              <code class="text-caption flex-1 text-truncate">
                {{ showToken[config.id] ? config.api_key : maskToken(config.api_key) }}
              </code>
              <v-btn
                icon
                size="x-small"
                variant="text"
                @click="showToken[config.id] = !showToken[config.id]"
              >
                <v-icon size="16">{{ showToken[config.id] ? mdiEyeOffOutline : mdiEyeOutline }}</v-icon>
              </v-btn>
              <v-btn
                icon
                size="x-small"
                variant="text"
                @click="openDetail(config)"
              >
                <v-icon size="20">{{ mdiInformationOutline }}</v-icon>
              </v-btn>
            </div>

            <v-spacer />

            <div class="d-flex gap-2">
              <v-btn
                variant="tonal"
                color="primary"
                size="small"
                rounded="pill"
                class="text-none flex-1"
                @click="openEditDialog(config)"
              >
                <v-icon start size="16">{{ mdiPencilOutline }}</v-icon>
                {{ $t('api.edit') }}
              </v-btn>
              <v-btn
                variant="tonal"
                color="error"
                size="small"
                rounded="pill"
                class="text-none"
                @click="selectedConfig = config; deleteDialog = true"
              >
                <v-icon size="16">{{ mdiTrashCanOutline }}</v-icon>
              </v-btn>
            </div>
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>

    <!-- 添加/编辑弹窗 (空白模板供用户补充) -->
    <!-- 添加/编辑弹窗 -->
    <v-dialog v-model="dialog" width="60%" :scrim="true" persistent scrollable>
      <v-card rounded="xl" class="subscription-dialog shadow-premium">
        <div class="dialog-header pa-6 d-flex align-center justify-space-between">
          <div>
            <h2 class="text-h5 font-weight-bold gradient-text">
              {{ selectedConfig ? $t('api.dialog_edit_title') : $t('api.dialog_add_title') }}
            </h2>
            <p class="text-caption text-medium-emphasis mt-1">
              {{ selectedConfig ? $t('api.dialog_edit_sub') : $t('api.dialog_add_sub') }}
            </p>
          </div>
          <v-btn :icon="mdiClose" variant="text" color="error" rounded="pill" @click="dialog = false"></v-btn>
        </div>
        
        <v-divider />

        <v-card-text class="pa-8 custom-scrollbar" style="max-height: 70vh;">
          <div class="d-flex flex-column gap-8 w-100">
            <!-- 基础信息 -->
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="primary" class="mr-2">{{ mdiKeyVariant }}</v-icon>
                {{ $t('api.base_url') }}
              </h3>
              <div class="d-flex flex-column gap-6 w-100">
                <div class="d-flex gap-4 w-100">
                  <v-text-field
                    v-model="form.name"
                    :label="$t('api.alias')"
                    variant="outlined"
                    rounded="lg"
                    color="primary"
                    :prepend-inner-icon="mdiLabelOutline"
                    persistent-hint
                    :hint="$t('api.alias_hint')"
                    style="flex: 1"
                    hide-details="auto"
                  />
                  <v-select
                    v-model="form.api_type"
                    :items="providers"
                    :label="$t('api.provider')"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    color="primary"
                    :prepend-inner-icon="mdiApps"
                    style="flex: 1"
                  />
                </div>

                <v-text-field
                  v-model="form.base_url"
                  :label="$t('api.base_url')"
                  variant="outlined"
                  rounded="lg"
                  color="primary"
                  :prepend-inner-icon="mdiLinkVariant"
                  persistent-hint
                  :hint="$t('api.base_url_hint')"
                  class="w-100"
                  hide-details="auto"
                />

                <v-text-field
                  v-model="form.api_key"
                  :label="$t('api.api_key')"
                  variant="outlined"
                  rounded="lg"
                  color="primary"
                  :prepend-inner-icon="mdiShieldKeyOutline"
                  type="password"
                  persistent-hint
                  :hint="$t('api.api_key_hint')"
                  class="w-100"
                  hide-details="auto"
                />
              </div>
            </section>

            <!-- OpenAI 特定配置 -->
            <template v-if="form.api_type === 'openai'">
              <v-divider />
              <section>
                <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                  <v-icon color="secondary" class="mr-2">{{ mdiBrain }}</v-icon>
                  {{ $t('api.ai_model_params') }}
                </h3>
                <div class="d-flex flex-column gap-6 w-100">
                  <div class="d-flex align-center gap-2 w-100">
                    <v-combobox
                      v-model="form.settings.model"
                      :items="availableModels"
                      :label="$t('api.model')"
                      variant="outlined"
                      density="comfortable"
                      rounded="lg"
                      color="primary"
                      class="flex-1"
                      hide-details
                    />
                    <v-btn
                      variant="tonal"
                      color="primary"
                      :icon="mdiRefresh"
                      rounded="lg"
                      height="48"
                      :loading="fetchingModels"
                      @click="fetchModels"
                    >
                      <v-icon>{{ mdiRefresh }}</v-icon>
                      <v-tooltip activator="parent">{{ $t('api.fetch_models') }}</v-tooltip>
                    </v-btn>
                  </div>
                  <div class="d-flex gap-4 w-100">
                    <v-text-field
                      v-model.number="form.settings.max_tokens"
                      :label="$t('api.max_tokens')"
                      type="number"
                      variant="outlined"
                      rounded="lg"
                      color="primary"
                      :prepend-inner-icon="mdiFormatTextWrappingOverflow"
                      style="flex: 1"
                      hide-details="auto"
                    />
                    <v-text-field
                      v-model.number="form.settings.rpm"
                      :label="$t('api.rpm')"
                      type="number"
                      variant="outlined"
                      rounded="lg"
                      color="primary"
                      :prepend-inner-icon="mdiSpeedometer"
                      style="flex: 1"
                      hide-details="auto"
                    />
                  </div>
                </div>
              </section>
            </template>

            <!-- 通用设置 -->
            <v-divider />
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="info" class="mr-2">{{ mdiCogOutline }}</v-icon>
                {{ $t('api.general_policy') }}
              </h3>
              <div class="d-flex flex-column gap-6 w-100">
                <div class="d-flex gap-4 w-100">
                  <v-text-field
                    v-model.number="form.timeout_seconds"
                    :label="$t('api.timeout')"
                    type="number"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    color="primary"
                    :prepend-inner-icon="mdiClockOutline"
                    style="flex: 1"
                  />
                  <div class="d-flex align-center justify-space-between px-4 border rounded-lg" style="flex: 1; height: 48px;">
                    <span class="text-body-2 text-medium-emphasis">{{ $t('api.retry_enabled') }}</span>
                    <v-switch v-model="form.retry_enabled" color="primary" hide-details density="compact" />
                  </div>
                </div>

                <div v-if="form.retry_enabled" class="d-flex gap-4 w-100">
                  <v-text-field
                    v-model.number="form.retry_count"
                    :label="$t('api.retry_count')"
                    type="number"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    color="primary"
                    :prepend-inner-icon="mdiRefresh"
                    style="flex: 1"
                  />
                  <v-text-field
                    v-model.number="form.retry_interval_ms"
                    :label="$t('api.retry_interval')"
                    type="number"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    color="primary"
                    :prepend-inner-icon="mdiTimerOutline"
                    style="flex: 1"
                  />
                </div>

                <!-- DeepLX 特定提示 -->
                <div v-if="form.api_type === 'deeplx'" class="w-100">
                  <v-alert type="info" variant="tonal" density="compact" rounded="lg" :icon="mdiInformationOutline">
                    {{ $t('api.deeplx_tip') }}
                  </v-alert>
                </div>
              </div>
            </section>
          </div>
        </v-card-text>

        <v-divider />

        <v-card-actions class="pa-6">
          <v-spacer />
          <v-btn variant="text" rounded="pill" class="px-6" @click="dialog = false">{{ $t('api.cancel') }}</v-btn>
          <v-btn
            color="primary"
            class="px-10 font-weight-bold btn-premium"
            rounded="pill"
            elevation="4"
            :loading="saving"
            @click="saveConfig"
          >
            {{ $t('api.save') }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 密钥详情弹窗 -->
    <v-dialog v-model="detailDialog" max-width="450">
      <v-card v-if="detailConfig" rounded="xl" class="pa-2">
        <v-card-title class="d-flex align-center gap-3 pa-4">
          <v-avatar :color="getProviderInfo(detailConfig.api_type).color" size="32" rounded="lg">
            <v-icon color="white" size="16">{{ getProviderInfo(detailConfig.api_type).icon }}</v-icon>
          </v-avatar>
          <span class="text-h6 font-weight-bold">{{ detailConfig.name }}</span>
          <v-spacer />
          <v-btn :icon="mdiClose" variant="text" size="small" @click="detailDialog = false"></v-btn>
        </v-card-title>
        
        <v-card-text class="pa-4 bg-surface rounded-lg mx-4 mb-4 border thin">
          <div class="d-flex flex-column gap-4">
            <div class="detail-item">
              <label class="text-caption text-medium-emphasis d-block mb-1">{{ $t('api.endpoint') }}</label>
              <div class="text-body-2 font-weight-medium d-flex align-center gap-2">
                <v-icon size="14">{{ mdiLinkVariant }}</v-icon>
                <span class="text-truncate">{{ detailConfig.base_url || $t('api.default_endpoint') }}</span>
              </div>
            </div>
            
            <div class="detail-item" v-if="(detailConfig.settings as any).model">
              <label class="text-caption text-medium-emphasis d-block mb-1">{{ $t('api.ai_model') }}</label>
              <div class="text-body-2 font-weight-medium d-flex align-center gap-2">
                <v-icon size="14">{{ mdiBrain }}</v-icon>
                {{ (detailConfig.settings as any).model }}
              </div>
            </div>

            <div class="d-flex gap-8">
              <div class="detail-item flex-1">
                <label class="text-caption text-medium-emphasis d-block mb-1">{{ $t('api.timeout_label') }}</label>
                <div class="text-body-2 font-weight-medium">{{ detailConfig.timeout_seconds }} {{ $t('api.unit_seconds') }}</div>
              </div>
              <div class="detail-item flex-1">
                <label class="text-caption text-medium-emphasis d-block mb-1">{{ $t('api.auto_retry') }}</label>
                <div class="text-body-2 font-weight-medium">
                  {{ detailConfig.retry_enabled ? `${detailConfig.retry_count} ${t('api.retry_unit')}` : t('api.not_engaged') }}
                </div>
              </div>
            </div>
          </div>
        </v-card-text>

        <v-card-actions class="pa-4 pt-0">
          <v-btn block color="primary" variant="tonal" rounded="pill" @click="detailDialog = false">{{ $t('common.confirm') }}</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Token Usage Stats Dialog (ECharts) -->
    <v-dialog v-model="statsDialog" width="90%" max-width="1200" scrollable>
      <v-card rounded="xl" class="pa-6">
        <div class="d-flex align-center justify-space-between mb-6">
          <div class="d-flex align-center gap-3">
            <v-avatar color="primary" variant="tonal" size="40">
              <v-icon color="primary">{{ mdiPoll }}</v-icon>
            </v-avatar>
            <h3 class="text-h6 font-weight-bold">Token 消耗统计分析</h3>
          </div>
          <v-btn :icon="mdiClose" variant="text" @click="statsDialog = false"></v-btn>
        </div>

        <v-card-text class="pa-0 custom-scrollbar">
          <!-- Filters -->
          <div class="d-flex flex-wrap gap-3 mb-6 align-center">
            <v-select
              v-model="selectedFilterApi"
              :items="[{ name: '全部 API', id: null }, ...apiConfigs]"
              item-title="name"
              item-value="id"
              placeholder="API 筛选"
              variant="outlined"
              density="comfortable"
              hide-details
              style="width: 220px"
              rounded="lg"
            />
            <v-select
              v-model="selectedFilterModel"
              :items="availableModelsList"
              placeholder="模型筛选"
              variant="outlined"
              density="comfortable"
              hide-details
              style="width: 220px"
              rounded="lg"
            />
            <v-spacer />
            <v-btn-toggle
              v-model="selectedRange"
              color="primary"
              variant="tonal"
              density="compact"
              mandatory
              rounded="lg"
            >
              <v-btn value="week" size="small">周</v-btn>
              <v-btn value="month" size="small">月</v-btn>
              <v-btn value="year" size="small">年</v-btn>
              <v-btn value="all" size="small">全部</v-btn>
            </v-btn-toggle>
          </div>

          <v-row>
            <v-col cols="12" lg="8">
              <v-card variant="flat" border class="pa-4 h-100 rounded-xl">
                <p class="text-subtitle-2 font-weight-bold mb-4 d-flex align-center">
                  <v-icon size="18" class="mr-2" color="primary">{{ mdiChartTimelineVariant }}</v-icon>
                  消耗历史趋势
                </p>
                <div style="height: 400px;">
                  <v-chart class="chart" :option="lineOption" autoresize />
                </div>
              </v-card>
            </v-col>
            <v-col cols="12" lg="4">
              <div class="d-flex flex-column gap-4 h-100">
                <v-card variant="flat" border class="pa-4 rounded-xl flex-grow-1 d-flex flex-column">
                  <p class="text-subtitle-2 font-weight-bold mb-4">按模型分布</p>
                  <div class="flex-grow-1" style="min-height: 300px;">
                    <v-chart class="chart" :option="pieOption" autoresize />
                  </div>
                </v-card>
              </div>
            </v-col>
          </v-row>

          <v-row class="text-center mt-6">
            <v-col v-if="usageStats">
              <v-card variant="tonal" color="primary" class="pa-6 rounded-xl">
                <p class="text-subtitle-2 mb-2 font-weight-bold">所有服务的总计消耗 (Tokens)</p>
                <p class="text-h2 font-weight-black">{{ usageStats.total_tokens.toLocaleString() }}</p>
              </v-card>
            </v-col>
          </v-row>
        </v-card-text>
      </v-card>
    </v-dialog>

    <!-- 删除确认 -->
    <v-dialog v-model="deleteDialog" max-width="360">
      <v-card rounded="xl">
        <v-card-title class="pa-6 pb-2 text-body-1 font-weight-bold">{{ $t('api.confirm_delete') }}</v-card-title>
        <v-card-text class="pa-6 pt-2 text-body-2 text-medium-emphasis">
          {{ $t('api.delete_msg', { name: selectedConfig?.name }) }}
        </v-card-text>
        <v-card-actions class="pa-6 pt-0">
          <v-spacer />
          <v-btn variant="text" rounded="pill" @click="deleteDialog = false">{{ $t('api.cancel') }}</v-btn>
          <v-btn color="error" class="text-none font-weight-bold px-6" rounded="pill" @click="deleteConfig">{{ $t('api.confirm_btn') }}</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.api-key-card {
  transition: 
    transform var(--md-motion-duration-medium) var(--md-motion-easing-emphasized),
    box-shadow var(--md-motion-duration-medium) var(--md-motion-easing-emphasized) !important;
  display: flex;
  flex-direction: column;
  overflow: hidden; /* 防止展开时的内容溢出导致宽度抖动 */
}

.api-key-card:active {
  transform: scale(0.99); /* 较弱的卡片缩放反馈 */
  transition-duration: 100ms !important;
}
.api-key-card :deep(.v-card-text) {
  flex: 1;
  display: flex;
  flex-direction: column;
}
.api-key-card:hover {
  box-shadow: 0 4px 20px rgba(var(--v-theme-primary), 0.12) !important;
}
.token-row code {
  font-family: 'Roboto Mono', monospace;
  background: rgba(var(--v-theme-on-surface), 0.06);
  border-radius: 6px;
  padding: 4px 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 200px;
}
.gap-1 { gap: 4px; }
.gap-2 { gap: 8px; }
.gap-3 { gap: 12px; }
.gap-4 { gap: 16px; }
.gap-6 { gap: 24px; }
.gap-8 { gap: 32px; }

/* Premium Dialog Styles */
.subscription-dialog {
  max-height: 90vh;
  display: flex;
  flex-direction: column;
}
.dialog-header {
  background: rgba(var(--v-theme-surface), 0.8);
  backdrop-filter: blur(10px);
  position: sticky;
  top: 0;
  z-index: 10;
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
.btn-premium {
  letter-spacing: 0.5px;
  text-transform: none;
  background: linear-gradient(135deg, var(--v-theme-primary) 0%, var(--v-theme-secondary) 100%) !important;
}
.cursor-pointer {
  cursor: pointer;
}
.chart {
  height: 300px;
  width: 100%;
}
</style>
