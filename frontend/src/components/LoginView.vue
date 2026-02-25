<script setup lang="ts">
import { ref } from 'vue'
import { useTheme } from 'vuetify'

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
    error.value = '请填写用户名和密码'
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
        throw new Error(data.message || (isLogin.value ? '登录失败' : '注册失败'))
    }

    if (isLogin.value) {
      const data = await response.json()
      localStorage.setItem('token', data.token)
      localStorage.setItem('username', data.username)
      emit('auth-success')
    } else {
      // After registration, switch to login
      isLogin.value = true
      messageType.value = 'success'
      error.value = '注册成功，请登录'
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
    <!-- 极简主题切换 -->
    <div class="theme-toggle">
      <v-btn icon variant="text" @click="toggleTheme">
        <v-icon>{{ theme.global.current.value.dark ? 'mdi-weather-sunny' : 'mdi-weather-night' }}</v-icon>
      </v-btn>
    </div>

    <v-card width="100%" max-width="400" class="pa-6 elevation-4 rounded-xl">
      <v-card-title class="text-h5 font-weight-bold text-center mb-4">
        {{ isLogin ? '登录' : '注册账号' }}
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
          label="用户名"
          variant="outlined"
          prepend-inner-icon="mdi-account"
          class="mb-2"
          rounded="lg"
        ></v-text-field>

       

        <v-text-field
          v-model="password"
          label="密码"
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
          {{ isLogin ? '立即登录' : '创建账号' }}
        </v-btn>

        <div class="d-flex align-center justify-center text-body-2 mt-4">
          <span class="text-medium-emphasis">
            {{ isLogin ? '还没有账号？' : '已有账号？' }}
          </span>
          <v-btn
            variant="text"
            color="primary"
            class="text-none font-weight-bold px-1"
            density="compact"
            @click="isLogin = !isLogin"
          >
            {{ isLogin ? '现在注册' : '返回登录' }}
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
