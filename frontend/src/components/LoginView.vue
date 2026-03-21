<script setup lang="ts">
import { ref } from 'vue'
import { useTheme } from 'vuetify'
import { useI18n } from 'vue-i18n'

const { t, locale } = useI18n()

const setLanguage = (lang: string) => {
  locale.value = lang
  localStorage.setItem('locale', lang)
}

const theme = useTheme()
const isLogin = ref(true)
const loading = ref(false)
const error = ref('')
const messageType = ref<'error' | 'success'>('error')

const username = ref('')
const password = ref('')


const toggleTheme = () => {
  theme.global.name.value = theme.global.current.value.dark ? 'light' : 'dark'
}

const emit = defineEmits(['auth-success'])

const handleSubmit = async () => {
  if (!username.value || !password.value) {
    error.value = t('auth.required')
    return
  }

  loading.value = true
  error.value = ''
  messageType.value = 'error'

  try {
    const endpoint = isLogin.value ? '/api/user/login' : '/api/user/register'
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        username: username.value,
        password: password.value,
      }),
    })

    if (!response.ok) {
        const data = await response.json().catch(() => ({}))
        throw new Error(data.message || (isLogin.value ? t('auth.login_failed') : t('auth.register_failed')))
    }

    if (isLogin.value) {
      const data = await response.json()
      localStorage.setItem('token', data.token)
      localStorage.setItem('username', data.username)

      // Apply theme mode configured by backend (`app_mode`: true for dark mode)
      if (data.app_mode !== undefined && data.app_mode !== null) {
        theme.global.name.value = data.app_mode ? 'dark' : 'light'
      }

      // Store other settings that might be useful
      if (data.log_num_limit) {
        localStorage.setItem('log_num_limit', data.log_num_limit.toString())
      }

      emit('auth-success')
    } else {
      // After registration, switch to login
      isLogin.value = true
      messageType.value = 'success'
      error.value = t('auth.reg_success')
      password.value = ''
    }
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <v-container class="fill-height d-flex align-center justify-center login-view-bg">
    <!-- 极简主题/语言切换 -->
    <div class="theme-toggle d-flex align-center">
      <!-- Compatible Language Select (Native fallback) -->
      <div class="lang-select-wrapper mr-2">
        <v-icon size="small" class="lang-icon">mdi-translate</v-icon>
        <select 
          v-model="locale" 
          class="lang-native-select"
          @change="setLanguage(locale)"
        >
          <option value="zh">简体中文</option>
          <option value="en">English</option>
        </select>
        <v-icon size="small" class="lang-arrow">mdi-chevron-down</v-icon>
      </div>
      <v-btn icon variant="text" @click="toggleTheme">
        <v-icon>{{ theme.global.current.value.dark ? 'mdi-weather-sunny' : 'mdi-weather-night' }}</v-icon>
      </v-btn>
    </div>

    <v-card width="100%" max-width="400" class="pa-6 elevation-4 rounded-xl">
      <v-card-title class="text-h5 font-weight-bold text-center mb-4">
        {{ isLogin ? t('auth.login') : t('auth.register') }}
      </v-card-title>

      <v-alert
        v-if="error"
        :type="messageType"
        variant="tonal"
        density="compact"
        class="mb-4 rounded-lg text-caption"
        closable
        @click:close="error = ''"
      >
        {{ error }}
      </v-alert>

      <v-form @submit.prevent="handleSubmit">
        <v-text-field
          v-model="username"
          :label="t('auth.username')"
          variant="outlined"
          prepend-inner-icon="mdi-account"
          class="mb-2"
          rounded="lg"
        ></v-text-field>

       

        <v-text-field
          v-model="password"
          :label="t('auth.password')"
          type="password"
          variant="outlined"
          prepend-inner-icon="mdi-lock"
          class="mb-4"
          rounded="lg"
        ></v-text-field>

        <v-btn
          type="submit"
          color="primary"
          block
          size="large"
          class="text-none font-weight-bold mb-4"
          rounded="lg"
          :loading="loading"
        >
          {{ isLogin ? t('auth.login_btn') : t('auth.register_btn') }}
        </v-btn>

        <div class="d-flex align-center justify-center text-body-2 mt-4">
          <span class="text-medium-emphasis">
            {{ isLogin ? t('auth.no_account') : t('auth.has_account') }}
          </span>
          <v-btn
            variant="text"
            color="primary"
            class="text-none font-weight-bold px-1"
            density="compact"
            @click="isLogin = !isLogin"
          >
            {{ isLogin ? t('auth.register_now') : t('auth.back_to_login') }}
          </v-btn>
        </div>
      </v-form>
    </v-card>
  </v-container>
</template>

<style scoped>
.login-view-bg {
  background-color: var(--md-sys-color-background);
  background-image: 
    radial-gradient(circle at 0% 0%, rgba(var(--md-sys-color-primary-rgb), 0.08) 0%, transparent 50%),
    radial-gradient(circle at 100% 100%, rgba(var(--md-sys-color-secondary-rgb), 0.05) 0%, transparent 50%);
}

.theme-toggle {
  position: absolute;
  top: 16px;
  right: 16px;
  z-index: 100;
}
</style>

<style scoped>
.lang-select-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  background: rgba(var(--v-theme-primary), 0.08);
  border-radius: 20px;
  padding: 0 12px;
  height: 32px;
  min-width: 140px;
  border: 1px solid rgba(var(--v-theme-primary), 0.1);
}

.lang-native-select {
  appearance: none;
  background: transparent;
  border: none;
  outline: none;
  font-size: 0.875rem;
  font-weight: 500;
  width: 100%;
  padding: 0 24px 0 26px;
  cursor: pointer;
  z-index: 1;
  color: inherit;
}

.lang-icon {
  position: absolute;
  left: 10px;
  pointer-events: none;
  opacity: 0.75;
}

.lang-arrow {
  position: absolute;
  right: 10px;
  pointer-events: none;
  opacity: 0.75;
}

.lang-native-select option {
  background: white; /* 登录页背景较浅 */
  color: #333;
}
</style>
