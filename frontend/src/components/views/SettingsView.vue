<script setup lang="ts">
import { ref } from 'vue'
import { useTheme } from 'vuetify'

const theme = useTheme()
const isDark = ref(theme.global.current.value.dark)

const toggleTheme = () => {
  isDark.value = !isDark.value
  theme.global.name.value = isDark.value ? 'dark' : 'light'
}

const language = ref('zh-CN')
const languages = [
  { title: '简体中文', value: 'zh-CN' },
  { title: 'English', value: 'en-US' },
  { title: '日本語', value: 'ja-JP' },
]

const syncInterval = ref(30)
const notificationsEnabled = ref(true)
const autoTranslate = ref(true)
const proxyEnabled = ref(false)
const proxyUrl = ref('')

const saving = ref(false)
const snackbar = ref(false)

const saveSettings = async () => {
  saving.value = true
  await new Promise(r => setTimeout(r, 800))
  saving.value = false
  snackbar.value = true
}
</script>

<template>
  <div class="settings-view">
    <div class="page-header mb-6">
      <h2 class="text-h5 font-weight-bold">设置</h2>
      <p class="text-body-2 text-medium-emphasis mt-1">管理应用程序偏好和系统配置</p>
    </div>

    <v-row>
      <!-- 外观设置 -->
      <v-col cols="12" md="6">
        <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-4">
          <v-card-item>
            <template #prepend>
              <v-icon color="primary" class="mr-2">mdi-palette-outline</v-icon>
            </template>
            <v-card-title class="text-body-1 font-weight-semibold">外观</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pt-4">
            <div class="d-flex align-center justify-space-between">
              <div>
                <p class="text-body-2 font-weight-medium">深色模式</p>
                <p class="text-caption text-medium-emphasis">切换应用明暗主题</p>
              </div>
              <v-switch
                v-model="isDark"
                color="primary"
                hide-details
                @update:model-value="toggleTheme"
              />
            </div>
            <v-divider class="my-3" />
            <v-select
              v-model="language"
              :items="languages"
              label="界面语言"
              variant="outlined"
              density="comfortable"
              rounded="lg"
              hide-details
              prepend-inner-icon="mdi-translate"
              color="primary"
            />
          </v-card-text>
        </v-card>
      </v-col>

      <!-- 同步设置 -->
      <v-col cols="12" md="6">
        <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-4">
          <v-card-item>
            <template #prepend>
              <v-icon color="primary" class="mr-2">mdi-sync</v-icon>
            </template>
            <v-card-title class="text-body-1 font-weight-semibold">同步</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pt-4">
            <p class="text-body-2 font-weight-medium mb-1">同步间隔（分钟）</p>
            <v-slider
              v-model="syncInterval"
              :min="5"
              :max="120"
              :step="5"
              color="primary"
              thumb-label
              class="mt-2"
            />
            <v-divider class="my-3" />
            <div class="d-flex align-center justify-space-between">
              <div>
                <p class="text-body-2 font-weight-medium">自动翻译</p>
                <p class="text-caption text-medium-emphasis">订阅文章自动翻译</p>
              </div>
              <v-switch v-model="autoTranslate" color="primary" hide-details />
            </div>
            <v-divider class="my-3" />
            <div class="d-flex align-center justify-space-between">
              <div>
                <p class="text-body-2 font-weight-medium">推送通知</p>
                <p class="text-caption text-medium-emphasis">新文章到达时通知</p>
              </div>
              <v-switch v-model="notificationsEnabled" color="primary" hide-details />
            </div>
          </v-card-text>
        </v-card>
      </v-col>

      <!-- 网络设置 -->
      <v-col cols="12">
        <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-4">
          <v-card-item>
            <template #prepend>
              <v-icon color="primary" class="mr-2">mdi-earth</v-icon>
            </template>
            <v-card-title class="text-body-1 font-weight-semibold">网络代理</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pt-4">
            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">启用代理</p>
                <p class="text-caption text-medium-emphasis">通过代理服务器访问订阅源</p>
              </div>
              <v-switch v-model="proxyEnabled" color="primary" hide-details />
            </div>
            <v-text-field
              v-model="proxyUrl"
              label="代理地址"
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
      </v-col>
    </v-row>

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

    <v-snackbar v-model="snackbar" :timeout="2000" location="bottom end" color="primary" rounded="pill">
      <v-icon start>mdi-check-circle-outline</v-icon>
      设置已保存
    </v-snackbar>
  </div>
</template>
