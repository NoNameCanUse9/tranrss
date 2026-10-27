<script setup lang="ts">
import { ref } from 'vue'
import LoginView from './components/LoginView.vue'
import HomeView from './components/HomeView.vue'

// 同步读取 token，避免已登录刷新时先闪现登录页。
const isLoggedIn = ref(!!localStorage.getItem('token'))

const handleAuthSuccess = () => {
  isLoggedIn.value = true
}
</script>

<template>
  <v-app>
    <LoginView v-if="!isLoggedIn" @auth-success="handleAuthSuccess" />
    <HomeView v-else @logout="isLoggedIn = false" />
  </v-app>
</template>

<style>
/* 全局基础样式 */
html, body {
  margin: 0;
  padding: 0;
  overflow: hidden !important;
  height: 100%;
  width: 100%;
}
#app {
  height: 100%;
}
</style>
