<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

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
const saving = ref(false)

const dialog = ref(false)
const deleteDialog = ref(false)
const selectedConfig = ref<ApiConfig | null>(null)
const showToken = ref<Record<number, boolean>>({})
const detailDialog = ref(false)
const detailConfig = ref<ApiConfig | null>(null)

const openDetail = (config: ApiConfig) => {
  detailConfig.value = config
  detailDialog.value = true
}

// 默认提供商配置，用于 UI 展示（图标、颜色等）
const providers = [
  { title: 'OpenAI', value: 'openai', icon: 'mdi-brain', color: '#10a37f' },
  { title: 'DeepLX', value: 'deeplx', icon: 'mdi-translate', color: '#0f2b46' },
]

// 初始表单状态
const form = ref({
  name: '',
  api_type: 'openai',
  api_key: '',
  base_url: '',
  settings: { max_tokens: 20480 } as Record<string, any>,
  timeout_seconds: 180,
  retry_count: 3,
  retry_interval_ms: 1000,
  retry_enabled: true,
})

// 从后端拉取配置
const fetchConfigs = async () => {
  loading.value = true
  try {
    const res = await fetch('/api/translate-configs', {
      headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
    })
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
    const res = await fetch('/api/user/setting', {
      headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
    })
    if (res.ok) {
      userSettings.value = await res.json()
    }
  } catch (e) {
    console.error('获取用户设置失败:', e)
  }
}

onMounted(() => {
  fetchConfigs()
  fetchUserSettings()
})

// 获取提供商的 UI 信息
const getProviderInfo = (type: string) =>
  providers.find(p => p.value === type) ?? { title: type, icon: 'mdi-key-outline', color: 'primary' }

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
    settings: { max_tokens: 20480 },
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
    settings: { ...config.settings as Record<string, any> },
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
    
    const res = await fetch(url, {
      method,
      headers: { 
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${localStorage.getItem('token')}` 
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
    const res = await fetch(`/api/translate-configs/${selectedConfig.value.id}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
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
</script>

<template>
  <div class="api-view">
    <div class="d-flex align-center justify-space-between mb-6">
      <div>
        <h2 class="text-h5 font-weight-bold">{{ $t('api.title') }}</h2>
        <p class="text-body-2 text-medium-emphasis mt-1">{{ $t('api.subtitle') }}</p>
      </div>
      <v-btn
        color="primary"
        rounded="pill"
        elevation="0"
        class="text-none font-weight-bold"
        @click="openAddDialog"
      >
        <v-icon start>mdi-plus</v-icon>
        {{ $t('api.add_btn') }}
      </v-btn>
    </div>

    <!-- 空状态 -->
    <v-card v-if="apiConfigs.length === 0" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-12">
      <v-icon size="64" color="primary" class="mb-4">mdi-key-outline</v-icon>
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
              <v-icon size="16" color="medium-emphasis">mdi-shield-key-outline</v-icon>
              <code class="text-caption flex-1 text-truncate">
                {{ showToken[config.id] ? config.api_key : maskToken(config.api_key) }}
              </code>
              <v-btn
                icon
                size="x-small"
                variant="text"
                @click="showToken[config.id] = !showToken[config.id]"
              >
                <v-icon size="16">{{ showToken[config.id] ? 'mdi-eye-off-outline' : 'mdi-eye-outline' }}</v-icon>
              </v-btn>
              <v-btn
                icon
                size="x-small"
                variant="text"
                @click="openDetail(config)"
              >
                <v-icon size="20">mdi-information-outline</v-icon>
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
                <v-icon start size="16">mdi-pencil-outline</v-icon>
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
                <v-icon size="16">mdi-trash-can-outline</v-icon>
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
          <v-btn icon="mdi-close" variant="text" color="error" rounded="pill" @click="dialog = false"></v-btn>
        </div>
        
        <v-divider />

        <v-card-text class="pa-8 custom-scrollbar" style="max-height: 70vh;">
          <div class="d-flex flex-column gap-8 w-100">
            <!-- 基础信息 -->
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="primary" class="mr-2">mdi-key-variant</v-icon>
                {{ $t('api.base_url') }}
              </h3>
              <div class="d-flex flex-column gap-6 w-100">
                <div class="d-flex gap-4 w-100">
                  <v-text-field
                    v-model="form.name"
                    :label="$t('api.alias')"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    color="primary"
                    prepend-inner-icon="mdi-label-outline"
                    persistent-hint
                    :hint="$t('api.alias_hint')"
                    style="flex: 1"
                  />
                  <v-select
                    v-model="form.api_type"
                    :items="providers"
                    :label="$t('api.provider')"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    color="primary"
                    prepend-inner-icon="mdi-apps"
                    style="flex: 1"
                  />
                </div>

                <v-text-field
                  v-model="form.base_url"
                  :label="$t('api.base_url')"
                  variant="outlined"
                  density="comfortable"
                  rounded="lg"
                  color="primary"
                  prepend-inner-icon="mdi-link-variant"
                  persistent-hint
                  :hint="$t('api.base_url_hint')"
                  class="w-100"
                />

                <v-text-field
                  v-model="form.api_key"
                  :label="$t('api.api_key')"
                  variant="outlined"
                  density="comfortable"
                  rounded="lg"
                  color="primary"
                  prepend-inner-icon="mdi-shield-key-outline"
                  type="password"
                  persistent-hint
                  :hint="$t('api.api_key_hint')"
                  class="w-100"
                />
              </div>
            </section>

            <!-- OpenAI 特定配置 -->
            <template v-if="form.api_type === 'openai'">
              <v-divider />
              <section>
                <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                  <v-icon color="secondary" class="mr-2">mdi-brain-outline</v-icon>
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
                      icon="mdi-refresh"
                      rounded="lg"
                      height="48"
                      :loading="fetchingModels"
                      @click="fetchModels"
                    >
                      <v-icon>mdi-refresh</v-icon>
                      <v-tooltip activator="parent">{{ $t('api.fetch_models') }}</v-tooltip>
                    </v-btn>
                  </div>
                  <div class="d-flex gap-4 w-100">
                    <v-text-field
                      v-model.number="form.settings.max_tokens"
                      :label="$t('api.max_tokens')"
                      type="number"
                      variant="outlined"
                      density="comfortable"
                      rounded="lg"
                      color="primary"
                      prepend-inner-icon="mdi-format-text-wrapping-overflow"
                      style="flex: 1"
                    />
                    <v-text-field
                      v-model.number="form.settings.rpm"
                      :label="$t('api.rpm')"
                      type="number"
                      variant="outlined"
                      density="comfortable"
                      rounded="lg"
                      color="primary"
                      prepend-inner-icon="mdi-speedometer"
                      style="flex: 1"
                    />
                  </div>
                </div>
              </section>
            </template>

            <!-- 通用设置 -->
            <v-divider />
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="info" class="mr-2">mdi-cog-outline</v-icon>
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
                    prepend-inner-icon="mdi-clock-outline"
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
                    prepend-inner-icon="mdi-refresh"
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
                    prepend-inner-icon="mdi-timer-outline"
                    style="flex: 1"
                  />
                </div>

                <!-- DeepLX 特定提示 -->
                <div v-if="form.api_type === 'deeplx'" class="w-100">
                  <v-alert type="info" variant="tonal" density="compact" rounded="lg" icon="mdi-information-outline">
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
          <v-btn icon="mdi-close" variant="text" size="small" @click="detailDialog = false"></v-btn>
        </v-card-title>
        
        <v-card-text class="pa-4 bg-surface rounded-lg mx-4 mb-4 border thin">
          <div class="d-flex flex-column gap-4">
            <div class="detail-item">
              <label class="text-caption text-medium-emphasis d-block mb-1">{{ $t('api.endpoint') }}</label>
              <div class="text-body-2 font-weight-medium d-flex align-center gap-2">
                <v-icon size="14">mdi-link-variant</v-icon>
                <span class="text-truncate">{{ detailConfig.base_url || $t('api.default_endpoint') }}</span>
              </div>
            </div>
            
            <div class="detail-item" v-if="(detailConfig.settings as any).model">
              <label class="text-caption text-medium-emphasis d-block mb-1">{{ $t('api.ai_model') }}</label>
              <div class="text-body-2 font-weight-medium d-flex align-center gap-2">
                <v-icon size="14">mdi-brain-outline</v-icon>
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
</style>
