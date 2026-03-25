<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useTheme } from 'vuetify'
import { useI18n } from 'vue-i18n'
import {
  mdiPaletteOutline,
  mdiDatabaseClock,
  mdiCogs,
  mdiApi,
  mdiTranslate,
  mdiTextShort,
  mdiDatabaseOutline,
  mdiFileImportOutline,
  mdiFileExportOutline,
  mdiEarth,
  mdiServerNetwork,
  mdiContentSaveOutline,
  mdiCheckCircleOutline,
  mdiContentCopy
} from '@mdi/js'

const { t } = useI18n()

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
const defaultApiId = ref<number | null>(null)
const customTransStyle = ref('')

const saving = ref(false)
const importing = ref(false)
const snackbar = ref(false)
const snackbarText = ref(t('settings.saved'))

const greaderUrl = ref(`${window.location.origin}/api/greader`)
const apiConfigs = ref<any[]>([])

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text)
  snackbarText.value = t('common.copy_success')
  snackbar.value = true
}

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
      if (data.default_api_id) defaultApiId.value = data.default_api_id
      if (data.custom_trans_style) customTransStyle.value = data.custom_trans_style
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
        default_api_id: defaultApiId.value,
        custom_trans_style: customTransStyle.value,
      })
    })
    
    localStorage.setItem('log_num_limit', logNumLimit.value.toString())
    snackbarText.value = t('settings.saved')
    snackbar.value = true
  } catch (e) {
    console.error('Save failed', e)
  } finally {
    saving.value = false
  }
}

const exportOPML = async () => {
  try {
    const res = await fetch('/api/feeds/opml', {
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
      const res = await fetch('/api/feeds/opml', {
        method: 'POST',
        headers: { Authorization: `Bearer ${localStorage.getItem('token')}` },
        body: formData
      })
      if (res.ok) {
        snackbarText.value = t('settings.import_success')
        snackbar.value = true
      } else {
        throw new Error('Import failed')
      }
    } catch (e) {
      console.error('Import failed', e)
      snackbarText.value = t('settings.import_failed')
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
      <h2 class="text-h5 font-weight-bold">{{ t('settings.title') }}</h2>
      <p class="text-body-2 text-medium-emphasis mt-1">{{ t('settings.subtitle') }}</p>
    </div>

    <div class="d-flex flex-column">
      <!-- 外观设置 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">{{ mdiPaletteOutline }}</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">{{ t('settings.appearance') }}</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">{{ t('settings.dark_mode') }}</p>
                <p class="text-caption text-medium-emphasis">{{ t('settings.dark_mode_desc') }}</p>
              </div>
              <v-switch
                v-model="isDark"
                color="success"
                hide-details
                @change="toggleTheme"
              />
            </div>
            
            <p class="text-body-2 font-weight-medium mb-2">{{ t('settings.queue_limit') }}</p>
            <v-text-field
              v-model="logNumLimit"
              type="number"
              placeholder="300"
              variant="outlined"
              density="comfortable"
              rounded="lg"
              hide-details
              :prepend-inner-icon="mdiDatabaseClock"
              color="primary"
            />
          </v-card-text>
        </v-card>

      <!-- 翻译样式设置 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">{{ mdiTranslate }}</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">翻译样式自定义 (CSS)</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <p class="text-body-2 font-weight-medium mb-2">已应用到 .trans-text 类的 CSS 样式</p>
            <v-textarea
              v-model="customTransStyle"
              placeholder="例如: display: block; font-style: italic; opacity: 0.6;"
              variant="outlined"
              density="comfortable"
              rounded="lg"
              auto-grow
              rows="5"
              color="primary"
              bg-color="surface"
              style="font-family: monospace;"
            >
              <template #prepend-inner>
                <v-icon color="medium-emphasis" class="mr-1">{{ mdiPaletteOutline }}</v-icon>
              </template>
            </v-textarea>
            <p class="text-caption text-medium-emphasis mt-2">
              提示：这是针对译文文本块的行内样式。可以使用标准的 CSS 属性，如 color, font-size, margin 等。
            </p>
          </v-card-text>
        </v-card>

      <!-- 服务设置 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">{{ mdiCogs }}</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">{{ t('settings.system_features') }}</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">Google Reader API</p>
                <p class="text-caption text-medium-emphasis">{{ t('settings.greader_desc') }}</p>
              </div>
              <v-switch v-model="greaderApi" color="success" hide-details />
            </div>

            <v-expand-transition>
              <div v-if="greaderApi" class="mt-n2 mb-6">
                <div class="d-flex align-center border-sm border-opacity-25 rounded-lg px-3 py-2 bg-surface">
                  <div class="flex-grow-1 overflow-hidden mr-2">
                    <p class="text-caption font-weight-bold text-primary mb-0">{{ t('settings.greader_url_tip') }}</p>
                    <code class="text-caption text-truncate d-block">{{ greaderUrl }}</code>
                  </div>
                  <v-btn
                    variant="tonal"
                    size="small"
                    color="primary"
                    :prepend-icon="mdiContentCopy"
                    class="text-none"
                    @click="copyToClipboard(greaderUrl)"
                  >
                    {{ t('common.copy') }}
                  </v-btn>
                </div>
              </div>
            </v-expand-transition>
            
            <div class="mb-4">
              <p class="text-body-2 font-weight-medium mb-2">{{ t('settings.global_default_api') }}</p>
              <div class="text-caption text-medium-emphasis mb-2">{{ t('settings.global_default_desc') }}</div>
              <v-select
                v-model="defaultApiId"
                :items="apiConfigs"
                item-title="name"
                item-value="id"
                :placeholder="t('settings.select_api_placeholder')"
                variant="outlined"
                density="comfortable"
                rounded="lg"
                hide-details
                clearable
                :prepend-inner-icon="mdiApi"
                color="primary"
              >
                <template #no-data>
                  <div class="px-4 py-2 text-caption text-medium-emphasis">{{ t('settings.no_api_configs') }}</div>
                </template>
              </v-select>
            </div>

            <v-divider class="my-4" />

            <div class="mb-4">
              <p class="text-body-2 font-weight-medium mb-2">{{ t('settings.default_translate_api') }}</p>
              <v-select
                v-model="translateApiId"
                :items="apiConfigs"
                item-title="name"
                item-value="id"
                :placeholder="t('settings.default_translate_api')"
                variant="outlined"
                density="comfortable"
                rounded="lg"
                hide-details
                clearable
                :prepend-inner-icon="mdiTranslate"
                color="primary"
              >
                <template #no-data>
                  <div class="px-4 py-2 text-caption text-medium-emphasis">{{ t('settings.no_api_configs') }}</div>
                </template>
              </v-select>
            </div>
            
            <div>
              <p class="text-body-2 font-weight-medium mb-2">{{ t('settings.default_summary_api') }}</p>
              <v-select
                v-model="summaryApiId"
                :items="apiConfigs"
                item-title="name"
                item-value="id"
                :placeholder="t('settings.default_summary_api')"
                variant="outlined"
                density="comfortable"
                rounded="lg"
                hide-details
                clearable
                :prepend-inner-icon="mdiTextShort"
                color="primary"
              >
                <template #no-data>
                  <div class="px-4 py-2 text-caption text-medium-emphasis">{{ t('settings.no_api_configs') }}</div>
                </template>
              </v-select>
            </div>

          </v-card-text>
        </v-card>

      <!-- 数据管理 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">{{ mdiDatabaseOutline }}</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">{{ t('settings.data_management') }}</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex flex-column flex-sm-row align-start align-sm-center justify-space-between gap-4">
              <div>
                <p class="text-body-2 font-weight-medium mb-1">{{ t('settings.opml_info') }}</p>
                <p class="text-caption text-medium-emphasis">{{ t('settings.opml_desc') }}</p>
              </div>
              <div class="d-flex" style="gap: 16px;">
                <v-btn
                  variant="outlined"
                  color="primary"
                  rounded="lg"
                  class="text-none font-weight-medium"
                  :prepend-icon="mdiFileImportOutline"
                  size="large"
                  min-width="200"
                  :loading="importing"
                  @click="triggerImport"
                >
                  {{ t('settings.import_opml') }}
                </v-btn>
                <v-btn
                  variant="tonal"
                  color="primary"
                  rounded="lg"
                  class="text-none font-weight-medium"
                  :prepend-icon="mdiFileExportOutline"
                  size="large"
                  min-width="200"
                  @click="exportOPML"
                >
                  {{ t('settings.export_opml') }}
                </v-btn>
              </div>
            </div>
          </v-card-text>
        </v-card>

      <!-- 网络设置 -->
      <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">{{ mdiEarth }}</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">{{ t('settings.proxy') }}</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">{{ t('settings.enable_proxy') }}</p>
                <p class="text-caption text-medium-emphasis">{{ t('settings.proxy_desc') }}</p>
              </div>
              <v-switch v-model="proxyEnabled" color="success" hide-details />
            </div>
            <p class="text-body-2 font-weight-medium mb-2" :class="{'text-medium-emphasis': !proxyEnabled}">{{ t('settings.proxy_url') }}</p>
            <v-text-field
              v-model="proxyUrl"
              placeholder="http://127.0.0.1:7890"
              variant="outlined"
              density="comfortable"
              rounded="lg"
              :disabled="!proxyEnabled"
              :prepend-inner-icon="mdiServerNetwork"
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
        <v-icon start>{{ mdiContentSaveOutline }}</v-icon>
        {{ t('settings.save') }}
      </v-btn>
    </div>

    <v-snackbar v-model="snackbar" :timeout="3000" location="bottom end" color="primary" rounded="pill">
      <v-icon start>{{ mdiCheckCircleOutline }}</v-icon>
      {{ snackbarText }}
    </v-snackbar>
  </div>
</template>
<style scoped>
.settings-view {
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  margin-top: 2rem;
}

/* 强制所有设置页面的开关在开启时显示明显的绿色 */
.settings-view :deep(.v-switch.v-selection-control--dirty) {
  --v-selection-control-color: #22c55e !important;
}

/* 针对 MD3 蓝图的特殊覆盖，强制滑块和轨道颜色 */
.settings-view :deep(.v-selection-control--dirty.v-switch--selected .v-switch__track) {
  background-color: #22c55e !important;
  border-color: #22c55e !important;
  opacity: 0.7 !important;
}

.settings-view :deep(.v-selection-control--dirty.v-switch--selected .v-switch__thumb) {
  background-color: #ffffff !important; /* 强制滑块为白色 */
  border-color: #22c55e !important;
}

/* 兼容深色模式下的亮度 */
.v-theme--dark .settings-view :deep(.v-selection-control--dirty.v-switch--selected .v-switch__track) {
  background-color: #4ade80 !important;
}

</style>
