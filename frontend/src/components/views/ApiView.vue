<script setup lang="ts">
import { ref, onMounted } from 'vue'

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
const loading = ref(false)
const saving = ref(false)

const dialog = ref(false)
const deleteDialog = ref(false)
const selectedConfig = ref<ApiConfig | null>(null)
const showToken = ref<Record<number, boolean>>({})
const expandedCards = ref<Record<number, boolean>>({})

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
  settings: {} as Record<string, any>,
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

onMounted(fetchConfigs)

// 获取提供商的 UI 信息
const getProviderInfo = (type: string) =>
  providers.find(p => p.value === type) ?? { title: type, icon: 'mdi-key-outline', color: 'primary' }

// 脱敏显示 Token
const maskToken = (token: string | null) => {
  if (!token) return '未设置'
  return token.length > 8 ? token.slice(0, 4) + '••••••••' + token.slice(-4) : '••••••••'
}

// 打开添加弹窗
const openAddDialog = () => {
  form.value = { 
    name: '', 
    api_type: 'openai', 
    api_key: '', 
    base_url: '', 
    settings: {},
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
</script>

<template>
  <div class="api-view">
    <div class="d-flex align-center justify-space-between mb-6">
      <div>
        <h2 class="text-h5 font-weight-bold">API 密钥</h2>
        <p class="text-body-2 text-medium-emphasis mt-1">管理翻译与AI服务的API密钥</p>
      </div>
      <v-btn
        color="primary"
        rounded="pill"
        elevation="0"
        class="text-none font-weight-bold"
        @click="openAddDialog"
      >
        <v-icon start>mdi-plus</v-icon>
        添加密钥
      </v-btn>
    </div>

    <!-- 空状态 -->
    <v-card v-if="apiConfigs.length === 0" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-12">
      <v-icon size="64" color="primary" class="mb-4">mdi-key-outline</v-icon>
      <h3 class="text-h6 mb-2">暂无 API 密钥</h3>
      <p class="text-body-2 text-medium-emphasis mb-6">添加翻译服务的API密钥以开始使用</p>
      <v-btn color="primary" rounded="pill" elevation="0" class="text-none" @click="openAddDialog">
        添加第一个密钥
      </v-btn>
    </v-card>

    <!-- 密钥列表 -->
    <v-row v-else>
      <v-col v-for="config in apiConfigs" :key="config.id" cols="12" md="6">
        <v-card rounded="xl" variant="flat" color="surface-variant" class="api-key-card">
          <v-card-text class="pa-5">
            <div class="d-flex align-start justify-space-between mb-3">
              <div class="d-flex align-center gap-3">
                <v-avatar :color="getProviderInfo(config.api_type).color" size="40" rounded="lg">
                  <v-icon color="white" size="20">{{ getProviderInfo(config.api_type).icon }}</v-icon>
                </v-avatar>
                <div>
                  <p class="text-body-1 font-weight-semibold">{{ config.name }}</p>
                  <p class="text-caption text-medium-emphasis">{{ getProviderInfo(config.api_type).title }}</p>
                </div>
              </div>
              <v-chip
                color="success"
                size="small"
                variant="tonal"
                class="text-none"
              >
                活跃
              </v-chip>
            </div>

            <v-divider class="my-3" />

            <div class="token-row d-flex align-center gap-2 mb-2">
              <v-icon size="16" color="medium-emphasis">mdi-shield-key-outline</v-icon>
              <code class="text-caption flex-1">
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
                :color="expandedCards[config.id] ? 'primary' : ''"
                @click="expandedCards[config.id] = !expandedCards[config.id]"
              >
                <v-icon size="18">{{ expandedCards[config.id] ? 'mdi-chevron-up' : 'mdi-chevron-down' }}</v-icon>
              </v-btn>
            </div>

            <v-expand-transition>
              <div v-show="expandedCards[config.id]" class="px-1 pt-1">
                <div v-if="config.base_url" class="d-flex align-center gap-2 mb-2 text-caption text-medium-emphasis">
                  <v-icon size="14">mdi-link-variant</v-icon>
                  <span class="text-truncate flex-1">{{ config.base_url }}</span>
                </div>

                <div class="d-flex flex-wrap align-center gap-4 text-caption text-medium-emphasis mb-3">
                  <span v-if="config.settings && (config.settings as any).model" class="d-flex align-center">
                    <v-icon size="14" class="mr-1">mdi-brain-outline</v-icon>
                    <span>{{ (config.settings as any).model }}</span>
                  </span>
                  <span class="d-flex align-center">
                    <v-icon size="14" class="mr-1">mdi-clock-outline</v-icon>
                    <span>{{ config.timeout_seconds }}s</span>
                  </span>
                  <span v-if="config.retry_enabled" class="d-flex align-center">
                    <v-icon size="14" class="mr-1">mdi-refresh</v-icon>
                    <span>{{ config.retry_count }}次</span>
                  </span>
                </div>
              </div>
            </v-expand-transition>

            <div class="d-flex gap-2 mt-2">
              <v-btn
                variant="tonal"
                color="primary"
                size="small"
                rounded="pill"
                class="text-none flex-1"
                @click="openEditDialog(config)"
              >
                <v-icon start size="16">mdi-pencil-outline</v-icon>
                编辑
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
    <v-dialog v-model="dialog" max-width="560" :scrim="true">
      <v-card rounded="xl" class="pa-4">
        <v-card-title class="text-h6 font-weight-bold">
          {{ selectedConfig ? '编辑配置' : '添加 API 配置' }}
        </v-card-title>
        
        <v-card-text>
          <!-- 这里留空，让用户补充具体的表单字段 -->
          

          <v-row dense>
             <v-col cols="12">
               <v-text-field
                 v-model="form.name"
                 label="配置名称"
                 variant="outlined"
                 rounded="lg"
                 hide-details
               />
             </v-col>
             <v-col cols="12">
               <v-select
                 v-model="form.api_type"
                 :items="providers"
                 label="类型"
                 variant="outlined"
                 rounded="lg"
                 hide-details
               />
             </v-col>
             <v-col cols="12">
               <v-text-field
                 v-model="form.base_url"
                 label="Api 端点"
                 variant="outlined"
                 rounded="lg"
                 hide-details
               />
             </v-col>
             <v-col cols="12">
               <v-text-field
                 v-model="form.api_key"
                 label="API 密钥 (可选)"
                 variant="outlined"
                 rounded="lg"
                 hide-details
                 type="password"
               />
             </v-col>

             <!-- OpenAI 特定配置 -->
             <template v-if="form.api_type === 'openai'">
               <v-divider class="my-4" />
               <v-col cols="12">
                 <v-text-field
                   v-model="form.settings.model"
                   label="模型 (Model)"
                   placeholder="gpt-4o-mini"
                   variant="outlined"
                   rounded="lg"
                   hide-details
                 />
               </v-col>
               <v-col cols="6">
                 <v-text-field
                   v-model.number="form.settings.max_tokens"
                   label="最大 token"
                   placeholder="4096"
                   type="number"
                   variant="outlined"
                   rounded="lg"
                   hide-details
                 />
               </v-col>
               <v-col cols="6">
                 <v-text-field
                   v-model.number="form.settings.rpm"
                   label="RPM (每分钟请求数)"
                   placeholder="3"
                   type="number"
                   variant="outlined"
                   rounded="lg"
                   hide-details
                 />
               </v-col>
             </template>

             <!-- 通用设置 -->
             <v-col cols="12">
               <div class="text-subtitle-2 font-weight-bold mt-4 mb-2">通用设置</div>
               <v-divider class="mb-4" />
             </v-col>

             <v-col cols="6">
               <v-text-field
                 v-model.number="form.timeout_seconds"
                 label="超时时间 (秒)"
                 type="number"
                 variant="outlined"
                 rounded="lg"
                 hide-details
               />
             </v-col>

             <v-col cols="6">
                <v-switch
                  v-model="form.retry_enabled"
                  label="启用重试"
                  color="primary"
                  hide-details
                  density="compact"
                />
             </v-col>

             <template v-if="form.retry_enabled">
               <v-col cols="6">
                 <v-text-field
                   v-model.number="form.retry_count"
                   label="重试次数"
                   type="number"
                   variant="outlined"
                   rounded="lg"
                   hide-details
                 />
               </v-col>
               <v-col cols="6">
                 <v-text-field
                   v-model.number="form.retry_interval_ms"
                   label="重试间隔 (ms)"
                   type="number"
                   variant="outlined"
                   rounded="lg"
                   hide-details
                 />
               </v-col>
             </template>

             <!-- DeepLX 特定配置 -->
             <template v-if="form.api_type === 'deeplx'">
               <v-col cols="12">
                 <v-alert
                   type="info"
                   variant="tonal"
                   density="compact"
                   class="text-caption mt-2"
                   rounded="lg"
                 >
                   <v-icon start size="16">mdi-information-outline</v-icon>
                   DeepLX 提示：如果你的 Token 已嵌入在 URL 中（例如 <code>?token=xxx</code>），则这里的“API 密钥”可以留空。
                 </v-alert>
               </v-col>
             </template>
          </v-row>
        </v-card-text>

        <v-card-actions class="pt-0">
          <v-spacer />
          <v-btn variant="text" rounded="pill" @click="dialog = false">取消</v-btn>
          <v-btn
            color="primary"
            class="px-6 font-weight-bold"
            rounded="pill"
            elevation="0"
            :loading="saving"
            @click="saveConfig"
          >
            保存配置
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 删除确认 -->
    <v-dialog v-model="deleteDialog" max-width="320">
      <v-card rounded="xl">
        <v-card-title class="text-body-1 font-weight-bold">确认删除?</v-card-title>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" variant="flat" rounded="pill" @click="deleteConfig">确定删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.api-key-card {
  transition: box-shadow 0.2s ease;
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
.gap-2 { gap: 8px; }
.gap-3 { gap: 12px; }
.gap-4 { gap: 16px; }
.gap-6 { gap: 24px; }
</style>
