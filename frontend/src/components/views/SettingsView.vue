<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useTheme } from 'vuetify'

const theme = useTheme()
const isDark = ref(theme.global.current.value.dark)

const toggleTheme = () => {
  theme.global.name.value = isDark.value ? 'dark' : 'light'
}

const logNumLimit = ref(300)
const proxyEnabled = ref(false)
const proxyUrl = ref('')
const greaderApi = ref(false)
const translateApiId = ref<number | null>(null)
const summaryApiId = ref<number | null>(null)

const saving = ref(false)
const importing = ref(false)
const snackbar = ref(false)
const snackbarText = ref('设置已保存')

const apiConfigs = ref<any[]>([])

const fetchApiConfigs = async () => {
  try {
    const res = await fetch('/api/translate-configs', {
      headers: { Authorization: `Bearer ${localStorage.getItem('token')}` }
    })
    if (res.ok) {
      apiConfigs.value = await res.json()
    }
  } catch (e) {
    console.error('Failed to load API configs', e)
  }
}

const loadSettings = async () => {
  try {
    const res = await fetch('/api/user/setting', {
      headers: { Authorization: `Bearer ${localStorage.getItem('token')}` }
    })
    if (res.ok) {
      const data = await res.json()
      if (data.app_mode !== undefined && data.app_mode !== null) {
        isDark.value = data.app_mode
        theme.global.name.value = data.app_mode ? 'dark' : 'light'
      }
      if (data.log_num_limit) logNumLimit.value = data.log_num_limit
      if (data.api_proxy !== undefined && data.api_proxy !== null) proxyEnabled.value = data.api_proxy
      if (data.api_proxy_url) proxyUrl.value = data.api_proxy_url
      if (data.greader_api !== undefined && data.greader_api !== null) greaderApi.value = data.greader_api
      if (data.translate_api_id) translateApiId.value = data.translate_api_id
      if (data.summary_api_id) summaryApiId.value = data.summary_api_id
    }
  } catch (e) {
    console.error('Failed to load settings', e)
  }
}

onMounted(async () => {
  await fetchApiConfigs()
  await loadSettings()
})

const saveSettings = async () => {
  saving.value = true
  try {
    await fetch('/api/user/setting', {
      method: 'PUT',
      headers: { 
        'Content-Type': 'application/json',
        Authorization: `Bearer ${localStorage.getItem('token')}`
      },
      body: JSON.stringify({
        app_mode: isDark.value,
        log_num_limit: Number(logNumLimit.value),
        api_proxy: proxyEnabled.value,
        api_proxy_url: proxyUrl.value,
        greader_api: greaderApi.value,
        translate_api_id: translateApiId.value,
        summary_api_id: summaryApiId.value,
      })
    })
    
    localStorage.setItem('log_num_limit', logNumLimit.value.toString())
    snackbarText.value = '设置已保存'
    snackbar.value = true
  } catch (e) {
    console.error('Save failed', e)
  } finally {
    saving.value = false
  }
}

const exportOPML = async () => {
  try {
    const res = await fetch('/api/subscriptions/opml', {
      headers: { Authorization: `Bearer ${localStorage.getItem('token')}` }
    })
    if (res.ok) {
      const blob = await res.blob()
      const url = window.URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `subscriptions_${new Date().toISOString().slice(0,10)}.opml`
      document.body.appendChild(a)
      a.click()
      window.URL.revokeObjectURL(url)
      document.body.removeChild(a)
    }
  } catch (e) {
    console.error('Export failed', e)
  }
}

const triggerImport = () => {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.opml,.xml'
  input.onchange = async (e: any) => {
    const file = e.target.files[0]
    if (!file) return
    
    importing.value = true
    const formData = new FormData()
    formData.append('file', file)
    
    try {
      const res = await fetch('/api/subscriptions/opml', {
        method: 'POST',
        headers: { Authorization: `Bearer ${localStorage.getItem('token')}` },
        body: formData
      })
      if (res.ok) {
        snackbarText.value = '订阅导入成功'
        snackbar.value = true
      } else {
        throw new Error('Import failed')
      }
    } catch (e) {
      console.error('Import failed', e)
      snackbarText.value = '导入失败，请检查文件格式'
      snackbar.value = true
    } finally {
      importing.value = false
    }
  }
  input.click()
}
</script>

<template>
  <div class="settings-view">
    <div class="page-header mb-6">
      <h2 class="text-h5 font-weight-bold">设置</h2>
      <p class="text-body-2 text-medium-emphasis mt-1">管理应用程序偏好和系统配置</p>
    </div>

    <div class="d-flex flex-column">
      <!-- 外观设置 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">mdi-palette-outline</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">外观</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">深色模式</p>
                <p class="text-caption text-medium-emphasis">切换应用明暗主题</p>
              </div>
              <v-switch
                v-model="isDark"
                color="primary"
                hide-details
                @change="toggleTheme"
              />
            </div>
            
            <p class="text-body-2 font-weight-medium mb-2">队列保留条数</p>
            <v-text-field
              v-model="logNumLimit"
              type="number"
              placeholder="300"
              variant="outlined"
              density="comfortable"
              rounded="lg"
              hide-details
              prepend-inner-icon="mdi-database-clock"
              color="primary"
            />
          </v-card-text>
        </v-card>

      <!-- 服务设置 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">mdi-cogs</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">系统功能</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">Google Reader API</p>
                <p class="text-caption text-medium-emphasis">支持第三方客户端同步</p>
              </div>
              <v-switch v-model="greaderApi" color="primary" hide-details />
            </div>
            
            <v-divider class="my-4" />

            <div class="mb-4">
              <p class="text-body-2 font-weight-medium mb-2">默认翻译 API</p>
              <v-select
                v-model="translateApiId"
                :items="apiConfigs"
                item-title="name"
                item-value="id"
                placeholder="选择默认翻译 API"
                variant="outlined"
                density="comfortable"
                rounded="lg"
                hide-details
                clearable
                prepend-inner-icon="mdi-translate"
                color="primary"
              >
                <template #no-data>
                  <div class="px-4 py-2 text-caption text-medium-emphasis">暂未配置API密钥</div>
                </template>
              </v-select>
            </div>
            
            <div>
              <p class="text-body-2 font-weight-medium mb-2">默认摘要 API</p>
              <v-select
                v-model="summaryApiId"
                :items="apiConfigs"
                item-title="name"
                item-value="id"
                placeholder="选择默认摘要 API"
                variant="outlined"
                density="comfortable"
                rounded="lg"
                hide-details
                clearable
                prepend-inner-icon="mdi-text-short"
                color="primary"
              >
                <template #no-data>
                  <div class="px-4 py-2 text-caption text-medium-emphasis">暂未配置API密钥</div>
                </template>
              </v-select>
            </div>

          </v-card-text>
        </v-card>

      <!-- 数据管理 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">mdi-database-outline</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">数据管理</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex flex-column flex-sm-row align-start align-sm-center justify-space-between gap-4">
              <div>
                <p class="text-body-2 font-weight-medium mb-1">OPML 订阅信息</p>
                <p class="text-caption text-medium-emphasis">支持与其他 RSS 客户端进行数据迁移和备份</p>
              </div>
              <div class="d-flex" style="gap: 16px;">
                <v-btn
                  variant="outlined"
                  color="primary"
                  rounded="lg"
                  class="text-none font-weight-medium"
                  prepend-icon="mdi-file-import-outline"
                  size="large"
                  min-width="200"
                  :loading="importing"
                  @click="triggerImport"
                >
                  导入 OPML
                </v-btn>
                <v-btn
                  variant="tonal"
                  color="primary"
                  rounded="lg"
                  class="text-none font-weight-medium"
                  prepend-icon="mdi-file-export-outline"
                  size="large"
                  min-width="200"
                  @click="exportOPML"
                >
                  导出 OPML
                </v-btn>
              </div>
            </div>
          </v-card-text>
        </v-card>

      <!-- 网络设置 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">mdi-earth</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">网络代理</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">启用代理</p>
                <p class="text-caption text-medium-emphasis">通过代理服务器访问订阅源</p>
              </div>
              <v-switch v-model="proxyEnabled" color="primary" hide-details />
            </div>
            <p class="text-body-2 font-weight-medium mb-2" :class="{'text-medium-emphasis': !proxyEnabled}">代理地址</p>
            <v-text-field
              v-model="proxyUrl"
              placeholder="http://127.0.0.1:7890"
              variant="outlined"
              density="comfortable"
              rounded="lg"
              :disabled="!proxyEnabled"
              prepend-inner-icon="mdi-server-network"
              hide-details
              color="primary"
            />
          </v-card-text>
        </v-card>
    </div>

    <div class="d-flex justify-end mt-2">
      <v-btn
        color="primary"
        rounded="pill"
        size="large"
        elevation="0"
        :loading="saving"
        class="text-none font-weight-bold px-8"
        @click="saveSettings"
      >
        <v-icon start>mdi-content-save-outline</v-icon>
        保存设置
      </v-btn>
    </div>

    <v-snackbar v-model="snackbar" :timeout="3000" location="bottom end" color="primary" rounded="pill">
      <v-icon start>mdi-check-circle-outline</v-icon>
      {{ snackbarText }}
    </v-snackbar>
  </div>
</template>
