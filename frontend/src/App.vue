<script setup lang="ts">
import { ref, onMounted } from 'vue'
import LoginView from './components/LoginView.vue'
import HomeView from './components/HomeView.vue'

import { isDemoMode } from './utils/demo'

const isLoggedIn = ref(false)

onMounted(() => {
  if (isDemoMode || localStorage.getItem('token')) {
    isLoggedIn.value = true
  }
})

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
  overflow-x: hidden;
}
</style>
