<script setup lang="ts">
import { ref, computed, shallowRef } from 'vue'
import { useTheme } from 'vuetify'
import SettingsView from './views/SettingsView.vue'
import ApiView from './views/ApiView.vue'
import SubscriptionView from './views/SubscriptionView.vue'
import QueueView from './views/QueueView.vue'

const theme = useTheme()
const isDark = ref(theme.global.current.value.dark)

const toggleTheme = () => {
  isDark.value = !isDark.value
  theme.global.name.value = isDark.value ? 'dark' : 'light'
}

// Navigation
const activeTab = ref(0)
const drawerOpen = ref(true)    // Desktop sidebar open/closed
const mobileDrawer = ref(false) // Mobile drawer

type NavItem = {
  icon: string
  activeIcon: string
  label: string
  badge?: number
}

const navItems: NavItem[] = [
  { icon: 'mdi-rss',                  activeIcon: 'mdi-rss-box',         label: '订阅' },
  { icon: 'mdi-format-list-checks',   activeIcon: 'mdi-format-list-checks',label: '队列', badge: 2 },
  { icon: 'mdi-key-variant',          activeIcon: 'mdi-key-variant',     label: 'API' },
  { icon: 'mdi-cog-outline',          activeIcon: 'mdi-cog',             label: '设置' },
]

const views = shallowRef([SubscriptionView, QueueView, ApiView, SettingsView])
const currentView = computed(() => views.value[activeTab.value])

const navigateTo = (index: number) => {
  activeTab.value = index
  mobileDrawer.value = false
}
</script>

<template>
  <v-app>
    <!-- ========== Mobile Navigation Drawer ========== -->
    <v-navigation-drawer
      v-model="mobileDrawer"
      class="d-flex d-md-none"
      temporary
      location="left"
      width="280"
      color="surface"
    >
      <!-- Logo -->
      <div class="pa-6 pb-4">
        <div class="d-flex align-center gap-3">
          <v-avatar class="logo-avatar" size="44" rounded="lg">
            <v-icon color="white" size="24">mdi-leaf</v-icon>
          </v-avatar>
          <div>
            <p class="text-h6 font-weight-bold" style="line-height:1.2; font-family: 'DM Serif Display', serif !important;">TranRSS</p>
            <p class="text-caption text-medium-emphasis">智能翻译专家</p>
          </div>
        </div>
      </div>

      <v-divider />

      <v-list nav class="pa-3 pt-4">
        <v-list-item
          v-for="(item, i) in navItems"
          :key="i"
          :active="activeTab === i"
          active-color="primary"
          rounded="xl"
          class="mb-1 text-body-2"
          @click="navigateTo(i)"
        >
          <template #prepend>
            <v-icon :color="activeTab === i ? 'primary' : 'on-surface-variant'">
              {{ activeTab === i ? item.activeIcon : item.icon }}
            </v-icon>
          </template>
          <v-list-item-title :class="activeTab === i ? 'text-primary' : 'text-on-surface-variant'">
            {{ item.label }}
          </v-list-item-title>
          <template v-if="item.badge" #append>
            <v-badge :content="item.badge" color="primary" inline />
          </template>
        </v-list-item>
      </v-list>

      <template #append>
        <v-divider />
        <div class="pa-4">
          <v-list-item
            prepend-icon="mdi-account-circle-outline"
            title="管理员"
            subtitle="admin@tranrss.app"
            rounded="xl"
            class="text-body-2"
          />
        </div>
      </template>
    </v-navigation-drawer>

    <!-- ========== Desktop Sidebar ========== -->
    <v-navigation-drawer
      v-model="drawerOpen"
      class="d-none d-md-flex"
      permanent
      :rail="!drawerOpen"
      rail-width="72"
      width="240"
      color="surface"
    >
      <!-- Logo -->
      <div :class="drawerOpen ? 'pa-5 pb-3' : 'pa-3 pb-2 d-flex justify-center'">
        <v-avatar class="logo-avatar cursor-pointer" :size="drawerOpen ? 48 : 42" rounded="lg" @click="drawerOpen = !drawerOpen">
          <v-icon color="white" :size="drawerOpen ? 26 : 24">mdi-leaf</v-icon>
        </v-avatar>
        <div v-if="drawerOpen" class="ml-3 mt-1">
          <p class="text-h6 font-weight-bold" style="line-height:1.2; font-family: 'DM Serif Display', serif !important;">TranRSS</p>
          <p class="text-caption text-medium-emphasis">智能翻译专家</p>
        </div>
      </div>

      <v-divider class="mx-3 mb-2" />

      <!-- Nav Items -->
      <v-list nav :class="drawerOpen ? 'pa-3' : 'pa-1'">
        <v-tooltip
          v-for="(item, i) in navItems"
          :key="i"
          :text="item.label"
          location="end"
          :disabled="drawerOpen"
        >
          <template #activator="{ props: tooltipProps }">
            <v-list-item
              v-bind="tooltipProps"
              :value="i"
              :active="activeTab === i"
              active-color="primary"
              rounded="xl"
              class="mb-1"
              :class="drawerOpen ? 'text-body-2 px-4' : 'justify-center px-1'"
              @click="navigateTo(i)"
            >
              <template #prepend>
                <div :class="drawerOpen ? 'mr-4' : 'mx-auto'">
                  <v-icon :color="activeTab === i ? 'primary' : 'on-surface-variant'" size="24">
                    {{ activeTab === i ? item.activeIcon : item.icon }}
                  </v-icon>
                </div>
              </template>
              <v-list-item-title v-if="drawerOpen" :class="activeTab === i ? 'text-primary font-weight-bold' : 'text-on-surface-variant'">
                {{ item.label }}
              </v-list-item-title>
              <template v-if="item.badge && drawerOpen" #append>
                <v-badge :content="item.badge" color="primary" inline />
              </template>
            </v-list-item>
          </template>
        </v-tooltip>
      </v-list>

      <template #append>
        <v-divider class="mx-3" />
        <div :class="drawerOpen ? 'pa-3' : 'pa-2 d-flex justify-center'">
          <v-list-item
            v-if="drawerOpen"
            prepend-icon="mdi-account-circle-outline"
            title="管理员"
            subtitle="admin@tranrss.app"
            rounded="xl"
            class="text-body-2"
          />
          <v-avatar v-else color="surface-variant" size="38" rounded="lg">
            <v-icon size="20">mdi-account-circle-outline</v-icon>
          </v-avatar>
        </div>
      </template>
    </v-navigation-drawer>

    <!-- ========== Top App Bar ========== -->
    <v-app-bar flat color="surface" class="border-b" height="64">
      <!-- Mobile menu button -->
      <v-app-bar-nav-icon
        class="d-flex d-md-none ml-1"
        @click="mobileDrawer = !mobileDrawer"
      />

      <!-- Desktop sidebar toggle -->
      <v-app-bar-nav-icon
        class="d-none d-md-flex ml-1"
        @click="drawerOpen = !drawerOpen"
      />

      <!-- Title breadcrumb -->
      <v-app-bar-title>
        <div class="d-flex align-center gap-1">
          <span class="text-h4 font-weight-bold">{{ navItems[activeTab]?.label }}</span>
        </div>
      </v-app-bar-title>

      <template #append>
        <!-- Theme toggle -->
        <v-btn icon variant="text" class="mr-1" @click="toggleTheme">
          <v-icon>{{ isDark ? 'mdi-weather-sunny' : 'mdi-weather-night' }}</v-icon>
        </v-btn>
        <!-- User avatar -->
        <v-avatar color="primary" size="34" class="mr-3 cursor-pointer">
          <span class="text-caption font-weight-bold text-on-primary">A</span>
        </v-avatar>
      </template>
    </v-app-bar>

    <!-- ========== Main Content ========== -->
    <v-main class="main-content">
      <v-container fluid class="pa-4 pa-sm-6 pa-md-8 max-content-width">
        <transition name="fade-slide" mode="out-in">
          <component :is="currentView" :key="activeTab" />
        </transition>
      </v-container>
    </v-main>

    <!-- ========== Mobile Bottom Navigation ========== -->
    <v-bottom-navigation
      v-model="activeTab"
      class="d-flex d-md-none"
      color="primary"
      bg-color="surface"
      elevation="8"
      grow
    >
      <v-btn v-for="(item, i) in navItems" :key="i" :value="i" class="text-none">
        <v-badge
          v-if="item.badge"
          :content="item.badge"
          color="error"
          floating
          offset-x="-4"
          offset-y="4"
        >
          <v-icon :color="activeTab === i ? 'primary' : 'on-surface-variant'">{{ activeTab === i ? item.activeIcon : item.icon }}</v-icon>
        </v-badge>
        <v-icon v-else :color="activeTab === i ? 'primary' : 'on-surface-variant'">{{ activeTab === i ? item.activeIcon : item.icon }}</v-icon>
        <span class="text-caption mt-1" :class="activeTab === i ? 'text-primary' : 'text-on-surface-variant'">{{ item.label }}</span>
      </v-btn>
    </v-bottom-navigation>
  </v-app>
</template>

<style scoped>
.border-b {
  border-bottom: 1px solid rgba(var(--v-border-color), 0.12) !important;
}

.main-content {
  background-color: rgb(var(--v-theme-background));
  min-height: 100vh;
}

.max-content-width {
  max-width: 1280px;
  margin: 0 auto;
}

.cursor-pointer {
  cursor: pointer;
}

.gap-1 { gap: 4px; }
.gap-3 { gap: 12px; }

/* Page transition */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(8px);
}
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
