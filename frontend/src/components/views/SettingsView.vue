<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
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
  mdiContentCopy,
  mdiAccountCog,
  mdiLockReset,
  mdiShieldLock,
  mdiAccountOutline,
  mdiKeyOutline,
  mdiPlus,
  mdiDeleteOutline,
  mdiClose,
  mdiKey,
  mdiAlertCircleOutline
} from '@mdi/js'
import { apiFetch } from '../../utils/api'

const { t } = useI18n()

// Account management state
const currentUsername = ref(localStorage.getItem('username') || 'admin')
const newUsername = ref(currentUsername.value)
const oldPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const allowRegistration = ref(true) // Global reg toggle

const theme = useTheme()
const isDark = computed({
  get: () => theme.global.current.value.dark,
  set: (val) => {
    const nextTheme = val ? 'dark' : 'light'
    theme.global.name.value = nextTheme
    localStorage.setItem('theme', nextTheme)
  }
})

const toggleTheme = () => {
  // isDark compute setter handles this
}

const logNumLimit = ref<number | string>(300)
const proxyEnabled = ref(false)
const proxyUrl = ref('')
const greaderApi = ref(false)
const feverApi = ref(false)
const translateApiId = ref<number | null>(null)
const summaryApiId = ref<number | null>(null)
const defaultApiId = ref<number | null>(null)
const customTransStyle = ref('')

const saving = ref(false)
const importing = ref(false)
const snackbar = ref(false)
const snackbarText = ref(t('settings.saved'))

const snackbarColor = ref<'success' | 'error'>('success')

const notify = (text: string, color: 'success' | 'error' = 'success') => {
  snackbarText.value = text
  snackbarColor.value = color
  snackbar.value = true
}

const greaderUrl = ref(`${window.location.origin}/api/greader`)
const feverUrl = ref(`${window.location.origin}/api/fever`)
const loadingConfigs = ref(true)
const apiConfigs = ref<any[]>([])

// Access Key management
interface AccessKeyInfo {
  id: number
  name: string
  key_prefix: string
  permissions: string[]
  created_at: string | null
  last_used_at: string | null
}

const accessKeys = ref<AccessKeyInfo[]>([])
const accessKeyDialog = ref(false)
const newKeyName = ref('')
const newKeyPerms = ref<string[]>([])
const createdKey = ref<string | null>(null)
const accessKeyLoading = ref(false)

const PERMISSION_GROUPS = [
  { labelKey: 'settings.perm_group_articles', resource: 'articles', actions: ['read', 'write'] },
  { labelKey: 'settings.perm_group_subscriptions', resource: 'subscriptions', actions: ['read', 'write'] },
  { labelKey: 'settings.perm_group_jobs', resource: 'jobs', actions: ['read', 'write'] },
  { labelKey: 'settings.perm_group_settings', resource: 'settings', actions: ['read', 'write'] },
  { labelKey: 'settings.perm_group_greader', resource: 'greader', actions: ['read', 'write'] },
]

const allConcretePerms = computed(() =>
  PERMISSION_GROUPS.flatMap(group => group.actions.map(action => `${group.resource}:${action}`))
)

const fetchAccessKeys = async () => {
  try {
    const res = await apiFetch('/api/user/access-keys')
    if (!res.ok) {
      notify(t('settings.keys_load_failed'), 'error')
      return
    }
    accessKeys.value = await res.json()
  } catch (e) {
    console.error('Failed to load access keys', e)
    notify(t('settings.keys_load_failed'), 'error')
  }
}

const openCreateKeyDialog = () => {
  newKeyName.value = ''
  newKeyPerms.value = []
  createdKey.value = null
  accessKeyDialog.value = true
}

const createAccessKey = async () => {
  if (!newKeyName.value) {
    notify(t('settings.key_name_required'), 'error')
    return
  }
  if (newKeyPerms.value.length === 0) {
    notify(t('settings.perm_select_at_least'), 'error')
    return
  }
  accessKeyLoading.value = true
  try {
    const res = await apiFetch('/api/user/access-keys', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: newKeyName.value,
        permissions: newKeyPerms.value,
      })
    })
    if (!res.ok) {
      notify(t('settings.key_create_failed'), 'error')
      return
    }
    const data = await res.json()
    createdKey.value = data.key
    await fetchAccessKeys()
  } catch (e) {
    console.error('Failed to create access key', e)
    notify(t('settings.key_create_failed'), 'error')
  } finally {
    accessKeyLoading.value = false
  }
}

const deleteAccessKey = async (id: number) => {
  if (!confirm(t('settings.key_delete_confirm'))) return
  try {
    const res = await apiFetch(`/api/user/access-keys/${id}`, { method: 'DELETE' })
    if (!res.ok) {
      notify(t('settings.key_delete_failed'), 'error')
      return
    }
    await fetchAccessKeys()
  } catch (e) {
    console.error('Failed to delete access key', e)
    notify(t('settings.key_delete_failed'), 'error')
  }
}

const togglePerm = (perm: string) => {
  // When every permission is granted through '*', expand it so the clicked chip can be revoked
  if (newKeyPerms.value.includes('*')) {
    newKeyPerms.value = allConcretePerms.value.filter(p => p !== perm)
    return
  }
  const idx = newKeyPerms.value.indexOf(perm)
  if (idx >= 0) {
    newKeyPerms.value.splice(idx, 1)
  } else {
    newKeyPerms.value.push(perm)
  }
}

const toggleAllPerms = () => {
  if (newKeyPerms.value.includes('*')) {
    newKeyPerms.value = []
  } else {
    newKeyPerms.value = ['*']
  }
}

const hasPerm = (perm: string) => newKeyPerms.value.includes(perm) || newKeyPerms.value.includes('*')

const legacyCopy = (text: string): boolean => {
  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.setAttribute('readonly', '')
  textarea.style.position = 'fixed'
  textarea.style.top = '-1000px'
  textarea.style.opacity = '0'
  document.body.appendChild(textarea)
  textarea.select()
  let ok = false
  try {
    ok = document.execCommand('copy')
  } catch {
    ok = false
  } finally {
    document.body.removeChild(textarea)
  }
  return ok
}

const copyToClipboard = async (text: string) => {
  try {
    if (!navigator.clipboard?.writeText) {
      throw new Error('Clipboard API unavailable')
    }
    await navigator.clipboard.writeText(text)
    notify(t('common.copy_success'))
  } catch {
    if (legacyCopy(text)) {
      notify(t('common.copy_success'))
    } else {
      notify(t('common.copy_failed'), 'error')
    }
  }
}

const fetchApiConfigs = async () => {
  loadingConfigs.value = true
  try {
    const res = await apiFetch('/api/translate-configs')
    if (res.ok) {
      apiConfigs.value = await res.json()
      
      // Auto-select if none set and only one available
      if (apiConfigs.value.length === 1) {
        if (defaultApiId.value === null) defaultApiId.value = apiConfigs.value[0].id
        if (translateApiId.value === null) translateApiId.value = apiConfigs.value[0].id
        const isOpenAi = ['openai', 'azure', 'deepseek', 'anthropic', 'gemini', 'ollama'].includes(apiConfigs.value[0].api_type.toLowerCase())
        if (summaryApiId.value === null && isOpenAi) summaryApiId.value = apiConfigs.value[0].id
      }
    }
  } catch (e) {
    console.error('Failed to load API configs', e)
  } finally {
    loadingConfigs.value = false
  }
}

const loadSettings = async () => {
  try {
    const res = await apiFetch('/api/user/setting')
    if (res.ok) {
      const data = await res.json()
      if (data.app_mode !== undefined && data.app_mode !== null) {
        // We only load it into the state if we want to sync, 
        // but here we let the global theme (from localStorage/main.ts) take precedence for the session.
        // If the user wants to sync with backend, they can see the difference or we can auto-apply once on login.
      }
      if (data.log_num_limit !== null && data.log_num_limit !== undefined) logNumLimit.value = data.log_num_limit
      if (data.api_proxy !== undefined && data.api_proxy !== null) proxyEnabled.value = data.api_proxy
      if (data.api_proxy_url) proxyUrl.value = data.api_proxy_url
      if (data.greader_api !== undefined && data.greader_api !== null) greaderApi.value = data.greader_api
      if (data.fever_api !== undefined && data.fever_api !== null) feverApi.value = data.fever_api
      if (data.translate_api_id) translateApiId.value = data.translate_api_id
      if (data.summary_api_id) summaryApiId.value = data.summary_api_id
      if (data.default_api_id) defaultApiId.value = data.default_api_id
      if (data.custom_trans_style) customTransStyle.value = data.custom_trans_style
    }
  } catch (e) {
    console.error('Failed to load settings', e)
  }
}

const fetchExtSettings = async () => {
    try {
        const res = await apiFetch('/api/user/registration-status')
        if (res.ok) {
            const data = await res.json()
            allowRegistration.value = data.allow
        }
    } catch (e) {
        console.error('Failed to load registration status', e)
    }
}

const toggleRegistration = async () => {
    const desired = allowRegistration.value
    try {
        const res = await apiFetch('/api/user/registration-toggle', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ allow: desired })
        })
        if (!res.ok) {
            throw new Error(await res.text())
        }
        notify(t('settings.saved'))
    } catch (e) {
        console.error('Failed to toggle registration', e)
        allowRegistration.value = !desired
        notify(t('settings.registration_update_failed'), 'error')
    }
}

const updateUsernameAction = async () => {
    if (!newUsername.value) return
    try {
        const res = await apiFetch('/api/user/username', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ new_username: newUsername.value })
        })
        if (!res.ok) {
            throw new Error(await res.text())
        }
        localStorage.setItem('username', newUsername.value)
        currentUsername.value = newUsername.value
        notify(t('settings.username_updated'))
    } catch (e) {
        console.error('Update username failed', e)
        notify(t('settings.username_update_failed'), 'error')
    }
}

const updatePasswordAction = async () => {
    if (!oldPassword.value || !newPassword.value) return
    if (newPassword.value !== confirmPassword.value) {
        notify(t('settings.password_mismatch'), 'error')
        return
    }
    try {
        const res = await apiFetch('/api/user/password', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ 
                old_password: oldPassword.value,
                new_password: newPassword.value 
            })
        })
        if (!res.ok) {
            throw new Error(await res.text())
        }
        oldPassword.value = ''
        newPassword.value = ''
        confirmPassword.value = ''
        notify(t('settings.password_updated'))
    } catch (e) {
        console.error('Update password failed', e)
        notify(t('settings.password_update_failed'), 'error')
    }
}

onMounted(async () => {
  await fetchApiConfigs()
  await loadSettings()
  await fetchExtSettings()
  await fetchAccessKeys()
})

const saveSettings = async () => {
  const limit = Number(logNumLimit.value)
  if (!Number.isInteger(limit) || limit <= 0) {
    notify(t('settings.invalid_queue_limit'), 'error')
    return
  }

  saving.value = true
  try {
    const res = await apiFetch('/api/user/setting', {
      method: 'PUT',
      headers: { 
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        app_mode: isDark.value,
        log_num_limit: limit,
        api_proxy: proxyEnabled.value,
        api_proxy_url: proxyUrl.value,
        greader_api: greaderApi.value,
        fever_api: feverApi.value,
        translate_api_id: translateApiId.value,
        summary_api_id: summaryApiId.value,
        default_api_id: defaultApiId.value,
        custom_trans_style: customTransStyle.value,
      })
    })

    if (!res.ok) {
      notify(t('settings.save_failed'), 'error')
      return
    }

    localStorage.setItem('theme', isDark.value ? 'dark' : 'light')
    localStorage.setItem('log_num_limit', limit.toString())
    notify(t('settings.saved'))
  } catch (e) {
    console.error('Save failed', e)
    notify(t('settings.save_failed'), 'error')
  } finally {
    saving.value = false
  }
}

const exportOPML = async () => {
  try {
    const res = await apiFetch('/api/feeds/opml')
    if (!res.ok) {
      notify(t('settings.opml_export_failed'), 'error')
      return
    }
    const blob = await res.blob()
    const url = window.URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `subscriptions_${new Date().toISOString().slice(0,10)}.opml`
    document.body.appendChild(a)
    a.click()
    window.URL.revokeObjectURL(url)
    document.body.removeChild(a)
  } catch (e) {
    console.error('Export failed', e)
    notify(t('settings.opml_export_failed'), 'error')
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
      const res = await apiFetch('/api/feeds/opml', {
        method: 'POST',
        body: formData
      })
      if (!res.ok) {
        throw new Error('Import failed')
      }
      notify(t('settings.import_success'))
    } catch (e) {
      console.error('Import failed', e)
      notify(t('settings.import_failed'), 'error')
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
        <!-- 账号管理 -->
        <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-10">
          <v-card-item class="pa-8 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-3" size="28">{{ mdiAccountCog }}</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">{{ t('settings.account_mgmt') }}</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-8">
            <!-- 修改用户名 -->
            <div class="mb-10">
                <p class="text-subtitle-2 font-weight-bold mb-4">{{ t('settings.change_username') }}</p>
                <div class="d-flex" style="gap: 16px;">
                    <div class="flex-grow-1">
                        <v-text-field
                            v-model="newUsername"
                            :placeholder="t('settings.new_username')"
                            variant="outlined"
                            density="comfortable"
                            rounded="lg"
                            hide-details
                            :prepend-inner-icon="mdiAccountOutline"
                            color="primary"
                            bg-color="surface"
                        />
                    </div>
                    <v-btn color="primary" rounded="xl" size="large" @click="updateUsernameAction" class="text-none">
                        {{ t('settings.update_username') }}
                    </v-btn>
                </div>
            </div>

            <v-divider class="mb-10" />

            <!-- 修改密码 -->
            <div>
                <p class="text-subtitle-2 font-weight-bold mb-4">{{ t('settings.change_password') }}</p>
                <v-text-field
                    v-model="oldPassword"
                    type="password"
                    :placeholder="t('settings.old_password')"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    class="mb-6"
                    hide-details
                    :prepend-inner-icon="mdiLockReset"
                    color="primary"
                    bg-color="surface"
                />
                
                <v-text-field
                    v-model="newPassword"
                    type="password"
                    :placeholder="t('settings.new_password')"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    class="mb-6"
                    hide-details
                    color="primary"
                    bg-color="surface"
                />
                
                <v-text-field
                    v-model="confirmPassword"
                    type="password"
                    :placeholder="t('settings.confirm_new_password')"
                    variant="outlined"
                    density="comfortable"
                    rounded="lg"
                    class="mb-8"
                    hide-details
                    color="primary"
                    bg-color="surface"
                />
                
                <div class="d-flex justify-end">
                    <v-btn
                        color="primary"
                        rounded="pill"
                        size="large"
                        variant="tonal"
                        min-width="180"
                        @click="updatePasswordAction"
                        class="text-none font-weight-bold"
                    >
                        {{ t('settings.reset_password') }}
                    </v-btn>
                </div>
            </div>
          </v-card-text>
        </v-card>

        <!-- API 访问密钥 -->
        <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-10">
          <v-card-item class="pa-8 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-3" size="28">{{ mdiKeyOutline }}</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">{{ t('settings.access_keys') }}</v-card-title>
            <template #append>
              <v-btn
                color="primary"
                variant="tonal"
                rounded="lg"
                size="small"
                :prepend-icon="mdiPlus"
                class="text-none"
                @click="openCreateKeyDialog"
              >
                {{ t('settings.create_key') }}
              </v-btn>
            </template>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-8">
            <p class="text-body-2 text-medium-emphasis mb-6">{{ t('settings.access_keys_desc') }} {{ t('settings.access_keys_save_hint') }}</p>

            <div v-if="accessKeys.length === 0" class="text-center py-8">
              <v-icon size="48" color="medium-emphasis" class="mb-3">{{ mdiKey }}</v-icon>
              <p class="text-body-1 text-medium-emphasis">{{ t('settings.no_keys') }}</p>
              <p class="text-caption text-medium-emphasis">{{ t('settings.no_keys_hint') }}</p>
            </div>

            <v-list v-else lines="two" density="comfortable" bg-color="surface" rounded="lg">
              <v-list-item
                v-for="key in accessKeys"
                :key="key.id"
                class="px-4"
              >
                <template #prepend>
                  <v-icon color="primary" size="20">{{ mdiKey }}</v-icon>
                </template>
                <v-list-item-title class="font-weight-medium">
                  {{ key.name }}
                  <code class="ml-2 text-caption" style="opacity: 0.6;">{{ key.key_prefix }}</code>
                </v-list-item-title>
                <v-list-item-subtitle>
                  <span v-if="key.permissions.includes('*')">{{ t('settings.perm_all') }}</span>
                  <span v-else>{{ key.permissions.join(', ') }}</span>
                  <span v-if="key.last_used_at" class="ml-2">· {{ t('settings.last_used') }}: {{ key.last_used_at }}</span>
                </v-list-item-subtitle>
                <template #append>
                  <v-btn
                    icon
                    variant="text"
                    size="small"
                    color="error"
                    @click="deleteAccessKey(key.id)"
                  >
                    <v-icon size="18">{{ mdiDeleteOutline }}</v-icon>
                  </v-btn>
                </template>
              </v-list-item>
            </v-list>
          </v-card-text>
        </v-card>

        <!-- 创建 Access Key 对话框 -->
        <v-dialog v-model="accessKeyDialog" max-width="560" persistent>
          <v-card rounded="xl">
            <v-card-title class="d-flex align-center justify-space-between pa-6 pb-2">
              <span class="text-h6 font-weight-bold">{{ t('settings.create_key_title') }}</span>
              <v-btn icon variant="text" size="small" @click="accessKeyDialog = false">
                <v-icon>{{ mdiClose }}</v-icon>
              </v-btn>
            </v-card-title>
            <v-divider />
            <v-card-text class="pa-6">
              <!-- 创建成功后显示完整 key -->
              <div v-if="createdKey" class="text-center py-4">
                <v-icon size="48" color="success" class="mb-3">{{ mdiCheckCircleOutline }}</v-icon>
                <p class="text-body-1 font-weight-bold mb-2">{{ t('settings.key_created') }}</p>
                <p class="text-caption text-medium-emphasis mb-4">{{ t('settings.key_created_hint') }}</p>
                <div class="d-flex align-center border-sm border-opacity-25 rounded-lg px-3 py-3 bg-surface mb-4">
                  <code class="text-body-2 flex-grow-1" style="word-break: break-all;">{{ createdKey }}</code>
                  <v-btn
                    variant="tonal"
                    size="small"
                    color="primary"
                    :prepend-icon="mdiContentCopy"
                    class="text-none ml-2"
                    @click="copyToClipboard(createdKey!)"
                  >
                    {{ t('common.copy') }}
                  </v-btn>
                </div>
                <v-btn
                  color="primary"
                  variant="flat"
                  rounded="lg"
                  class="text-none"
                  @click="accessKeyDialog = false"
                >
                  {{ t('settings.done') }}
                </v-btn>
              </div>

              <!-- 创建表单 -->
              <div v-else>
                <v-text-field
                  v-model="newKeyName"
                  :label="t('settings.key_name')"
                  :placeholder="t('settings.key_name_hint')"
                  variant="outlined"
                  density="comfortable"
                  rounded="lg"
                  class="mb-6"
                  hide-details
                  :prepend-inner-icon="mdiKeyOutline"
                  color="primary"
                  bg-color="surface"
                />

                <p class="text-subtitle-2 font-weight-bold mb-3">{{ t('settings.permissions') }}</p>
                <v-card variant="outlined" rounded="lg" class="mb-4">
                  <v-card-text class="pa-4">
                    <!-- 全选 -->
                    <div class="d-flex align-center justify-space-between mb-3 pb-3 border-b-sm border-opacity-10">
                      <span class="text-body-2 font-weight-medium">{{ t('settings.perm_all') }}</span>
                      <v-switch
                        :model-value="newKeyPerms.includes('*')"
                        color="primary"
                        hide-details
                        density="compact"
                        @update:model-value="toggleAllPerms"
                      />
                    </div>

                    <!-- 分组权限 -->
                    <div v-for="group in PERMISSION_GROUPS" :key="group.resource" class="mb-2">
                      <p class="text-caption text-medium-emphasis font-weight-bold mb-1">{{ t(group.labelKey) }}</p>
                      <div class="d-flex flex-wrap" style="gap: 8px;">
                        <v-chip
                          v-for="action in group.actions"
                          :key="`${group.resource}:${action}`"
                          :color="hasPerm(`${group.resource}:${action}`) ? 'primary' : 'default'"
                          :variant="hasPerm(`${group.resource}:${action}`) ? 'flat' : 'outlined'"
                          size="small"
                          class="text-none"
                          @click="togglePerm(`${group.resource}:${action}`)"
                        >
                          {{ action === 'read' ? t('settings.read_perm') : t('settings.write_perm') }}
                        </v-chip>
                      </div>
                    </div>
                  </v-card-text>
                </v-card>

                <v-alert
                  v-if="newKeyPerms.length === 0"
                  type="warning"
                  variant="tonal"
                  density="compact"
                  rounded="lg"
                  class="mb-4"
                  :icon="mdiAlertCircleOutline"
                >
                  {{ t('settings.perm_select_at_least') }}
                </v-alert>
              </div>
            </v-card-text>
            <v-card-actions v-if="!createdKey" class="pa-6 pt-0">
              <v-spacer />
              <v-btn
                variant="text"
                rounded="lg"
                class="text-none"
                @click="accessKeyDialog = false"
              >
                {{ t('common.cancel') }}
              </v-btn>
              <v-btn
                color="primary"
                variant="flat"
                rounded="lg"
                class="text-none"
                :loading="accessKeyLoading"
                :disabled="!newKeyName || newKeyPerms.length === 0"
                @click="createAccessKey"
              >
                {{ t('settings.create_key_submit') }}
              </v-btn>
            </v-card-actions>
          </v-card>
        </v-dialog>

        <!-- 安全设置 -->
        <v-card rounded="xl" variant="flat" color="surface-variant" class="mb-6">
          <v-card-item class="pa-6 pb-4">
            <template #prepend>
              <v-icon color="primary" class="mr-2">{{ mdiShieldLock }}</v-icon>
            </template>
            <v-card-title class="text-h6 font-weight-bold">{{ t('settings.security_settings') }}</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">{{ t('settings.allow_registration') }}</p>
                <p class="text-caption text-medium-emphasis">{{ t('settings.allow_registration_desc') }}</p>
              </div>
              <v-switch
                v-model="allowRegistration"
                color="success"
                hide-details
                @change="toggleRegistration"
              />
            </div>
          </v-card-text>
        </v-card>

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
            <v-card-title class="text-h6 font-weight-bold">{{ t('settings.trans_style_title') }}</v-card-title>
          </v-card-item>
          <v-divider />
          <v-card-text class="pa-6">
            <p class="text-body-2 font-weight-medium mb-2">{{ t('settings.trans_style_applied') }}</p>
            <v-textarea
              v-model="customTransStyle"
              :placeholder="t('settings.trans_style_placeholder')"
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
              {{ t('settings.trans_style_hint') }}
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

            <div class="d-flex align-center justify-space-between mb-4">
              <div>
                <p class="text-body-2 font-weight-medium">Fever API</p>
                <p class="text-caption text-medium-emphasis">{{ t('settings.fever_desc') }}</p>
              </div>
              <v-switch v-model="feverApi" color="success" hide-details />
            </div>

            <v-expand-transition>
              <div v-if="feverApi" class="mt-n2 mb-6">
                <div class="d-flex align-center border-sm border-opacity-25 rounded-lg px-3 py-2 bg-surface">
                  <div class="flex-grow-1 overflow-hidden mr-2">
                    <p class="text-caption font-weight-bold text-primary mb-0">{{ t('settings.fever_url_tip') }}</p>
                    <code class="text-caption text-truncate d-block">{{ feverUrl }}</code>
                  </div>
                  <v-btn
                    variant="tonal"
                    size="small"
                    color="primary"
                    :prepend-icon="mdiContentCopy"
                    class="text-none"
                    @click="copyToClipboard(feverUrl)"
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
                :loading="loadingConfigs"
                :prepend-inner-icon="mdiApi"
                color="primary"
              >
                <template #item="{ props, item }">
                  <v-list-item v-bind="props" :subtitle="item.raw.api_type.toUpperCase()">
                  </v-list-item>
                </template>
                <template #no-data>
                  <div class="pa-4 text-center">
                    <p class="text-caption text-medium-emphasis mb-2">{{ t('settings.no_api_configs') }}</p>
                    <p class="text-caption">{{ t('settings.add_api_config_hint') }}</p>
                  </div>
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
                :loading="loadingConfigs"
                :prepend-inner-icon="mdiTranslate"
                color="primary"
              >
                <template #item="{ props, item }">
                  <v-list-item v-bind="props" :subtitle="item.raw.api_type.toUpperCase()">
                  </v-list-item>
                </template>
                <template #no-data>
                  <div class="pa-4 text-center">
                    <p class="text-caption text-medium-emphasis mb-2">{{ t('settings.no_api_configs') }}</p>
                  </div>
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
                :loading="loadingConfigs"
                :prepend-inner-icon="mdiTextShort"
                color="primary"
              >
                <template #item="{ props, item }">
                  <v-list-item v-bind="props" :subtitle="item.raw.api_type.toUpperCase()">
                  </v-list-item>
                </template>
                <template #no-data>
                  <div class="pa-4 text-center">
                    <p class="text-caption text-medium-emphasis mb-2">{{ t('settings.no_api_configs') }}</p>
                  </div>
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
              <div class="opml-actions d-flex flex-wrap">
                <v-btn
                  variant="outlined"
                  color="primary"
                  rounded="lg"
                  class="text-none font-weight-medium"
                  :prepend-icon="mdiFileImportOutline"
                  size="large"

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

    <v-snackbar v-model="snackbar" :timeout="3000" location="bottom end" :color="snackbarColor" rounded="pill">
      <v-icon start>{{ snackbarColor === 'error' ? mdiAlertCircleOutline : mdiCheckCircleOutline }}</v-icon>
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

/* OPML 两个按钮原先固定 200px 宽，在 390px 视口下整行溢出 66px。 */
.opml-actions {
  gap: 16px;
}

.opml-actions :deep(.v-btn) {
  min-width: 200px;
}

@media (max-width: 599px) {
  .opml-actions {
    width: 100%;
  }

  .opml-actions :deep(.v-btn) {
    flex: 1 1 100%;
    min-width: 0 !important;
  }
}

</style>
