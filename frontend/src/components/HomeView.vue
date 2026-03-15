<script setup lang="ts">
import { ref, computed, shallowRef } from 'vue'
import { useTheme } from 'vuetify'
import SettingsView from './views/SettingsView.vue'
import ApiView from './views/ApiView.vue'
import SubscriptionView from './views/SubscriptionView.vue'
import QueueView from './views/QueueView.vue'
import ArticleView from './views/ArticleView.vue'
import { onMounted } from 'vue'

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
const openedGroups = ref(['articles'])

type NavItem = {
  icon: string
  activeIcon: string
  label: string
  badge?: number
}

const navItems = computed((): NavItem[] => {
  const badgeCount = activeJobsCount.value
  return [
    { icon: 'mdi-newspaper-variant-outline', activeIcon: 'mdi-newspaper-variant', label: '文章' },
    { icon: 'mdi-rss',                  activeIcon: 'mdi-rss-box',         label: '订阅' },
    { icon: 'mdi-text-box-outline',     activeIcon: 'mdi-text-box',          label: '日志', badge: badgeCount > 0 ? badgeCount : undefined },
    { icon: 'mdi-key-variant',          activeIcon: 'mdi-key-variant',     label: 'API' },
    { icon: 'mdi-cog-outline',          activeIcon: 'mdi-cog',             label: '设置' },
  ]
})

const views = shallowRef([ArticleView, SubscriptionView, QueueView, ApiView, SettingsView])
const currentView = computed(() => views.value[activeTab.value])

const navigateTo = (index: number) => {
  activeTab.value = index
  mobileDrawer.value = false
  if (index !== 0) {
    selectedFeedId.value = undefined
    filterRead.value = undefined
    filterStarred.value = undefined
  }
}

const emit = defineEmits(['logout'])

const subscriptions = ref<any[]>([])
const selectedFeedId = ref<number | undefined>(undefined)
const filterRead = ref<boolean | undefined>(undefined)
const filterStarred = ref<boolean | undefined>(undefined)

const fetchSubscriptions = async () => {
  try {
    const response = await fetch('/api/subscriptions', {
      headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
    })
    if (response.ok) {
      subscriptions.value = await response.json()
    }
  } catch (e) {
    console.error(e)
  }
}

onMounted(() => {
  fetchSubscriptions()
  fetchActiveJobsCount()
  setInterval(fetchActiveJobsCount, 30000)
})

const groupedSubscriptions = computed(() => {
  const groups: Record<string, any[]> = {}
  subscriptions.value.forEach(sub => {
    const cat = sub.category || '未分类'
    if (!groups[cat]) groups[cat] = []
    groups[cat].push(sub)
  })
  return groups
})

const articleListOpen = ref(true)

const selectFeed = (feedId?: number, isRead?: boolean, isStarred?: boolean) => {
  selectedFeedId.value = feedId
  filterRead.value = isRead
  filterStarred.value = isStarred
  activeTab.value = 0
  articleListOpen.value = true // 切换源时默认打开列表
}

const logout = () => {
  localStorage.removeItem('token')
  localStorage.removeItem('username')
  emit('logout')
}

const activeJobsCount = ref(0)
const fetchActiveJobsCount = async () => {
  try {
    const response = await fetch('/api/jobs', {
      headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
    })
    if (response.ok) {
      const data: { status: string }[] = await response.json()
      // Only count active or failing tasks
      activeJobsCount.value = data.filter(j => 
        ['pending', 'running', 'failed', 'retry'].includes(j.status.toLowerCase())
      ).length
    }
  } catch (e) {
    // ignore
  }
}
</script>

<template>
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

      <v-list nav class="pa-3 pt-4 overflow-y-auto" v-model:opened="openedGroups">
        <!-- Articles Group (Mobile) -->
        <v-list-group value="articles">
          <template #activator="{ props }">
            <v-list-item
              v-bind="props"
              :active="activeTab === 0 && !selectedFeedId && filterRead === undefined && filterStarred === undefined"
              active-color="primary"
              rounded="xl"
              class="mb-1 text-body-2"
              @click="navigateTo(0); selectFeed(undefined, undefined, undefined)"
            >
              <template #prepend>
                <v-icon :color="activeTab === 0 ? 'primary' : 'on-surface-variant'">
                  {{ activeTab === 0 ? 'mdi-newspaper-variant' : 'mdi-newspaper-variant-outline' }}
                </v-icon>
              </template>
              <v-list-item-title :class="activeTab === 0 ? 'text-primary' : 'text-on-surface-variant'">
                全部文章
              </v-list-item-title>
            </v-list-item>
          </template>

          <v-list-item 
            title="未读" 
            prepend-icon="mdi-circle-medium" 
            density="compact" 
            class="pl-6 mb-1 text-body-2" 
            :active="activeTab === 0 && filterRead === false" 
            @click="selectFeed(undefined, false)" 
          />
          <v-list-item 
            title="收藏" 
            prepend-icon="mdi-star-outline" 
            density="compact" 
            class="pl-6 mb-1 text-body-2" 
            :active="activeTab === 0 && filterStarred === true" 
            @click="selectFeed(undefined, undefined, true)" 
          />

          <v-list-group v-for="(subs, cat) in groupedSubscriptions" :key="cat" :value="cat">
            <template #activator="{ props }">
              <v-list-item v-bind="props" :title="cat" density="compact" class="text-caption" />
            </template>
            <v-list-item
              v-for="sub in subs"
              :key="sub.id"
              :title="sub.title"
              density="compact"
              class="pl-10 text-caption rounded-lg mb-1"
              :active="activeTab === 0 && selectedFeedId === sub.feedId"
              @click="selectFeed(sub.feedId)"
            />
          </v-list-group>
        </v-list-group>

        <!-- Other Items -->
        <v-list-item
          v-for="(item, i) in navItems.slice(1)"
          :key="i + 1"
          :active="activeTab === i + 1"
          active-color="primary"
          rounded="xl"
          class="mb-1 text-body-2"
          @click="navigateTo(i + 1)"
        >
          <template #prepend>
            <v-icon :color="activeTab === i + 1 ? 'primary' : 'on-surface-variant'">
              {{ activeTab === i + 1 ? item.activeIcon : item.icon }}
            </v-icon>
          </template>
          <v-list-item-title :class="activeTab === i + 1 ? 'text-primary' : 'text-on-surface-variant'">
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
      <v-list nav :class="drawerOpen ? 'pa-3' : 'pa-1'" v-model:opened="openedGroups" class="overflow-y-auto">
        <!-- Articles Group (Only Desktop expanded) -->
        <v-list-group value="articles" v-if="drawerOpen">
          <template #activator="{ props }">
            <v-list-item
              v-bind="props"
              :active="activeTab === 0 && !selectedFeedId && filterRead === undefined && filterStarred === undefined"
              active-color="primary"
              rounded="xl"
              class="mb-1 text-body-2 px-4"
              @click="navigateTo(0); selectFeed(undefined, undefined, undefined)"
            >
              <template #prepend>
                <v-icon :color="activeTab === 0 ? 'primary' : 'on-surface-variant'" class="mr-4">
                  {{ activeTab === 0 ? 'mdi-newspaper-variant' : 'mdi-newspaper-variant-outline' }}
                </v-icon>
              </template>
              <v-list-item-title :class="activeTab === 0 ? 'text-primary font-weight-bold' : 'text-on-surface-variant'">
                全部文章
              </v-list-item-title>
            </v-list-item>
          </template>

          <v-list-item 
            title="未读" 
            prepend-icon="mdi-circle-medium" 
            density="compact" 
            class="pl-10 mb-1 text-body-2 rounded-lg" 
            :active="activeTab === 0 && filterRead === false" 
            @click="selectFeed(undefined, false)" 
          />
          <v-list-item 
            title="收藏" 
            prepend-icon="mdi-star-outline" 
            density="compact" 
            class="pl-10 mb-1 text-body-2 rounded-lg" 
            :active="activeTab === 0 && filterStarred === true" 
            @click="selectFeed(undefined, undefined, true)" 
          />

          <v-list-group v-for="(subs, cat) in groupedSubscriptions" :key="cat" :value="cat">
            <template #activator="{ props }">
              <v-list-item v-bind="props" :title="cat" density="compact" class="text-caption opacity-80" />
            </template>
            <v-list-item
              v-for="sub in subs"
              :key="sub.id"
              :title="sub.title"
              density="compact"
              class="pl-10 text-caption rounded-lg mb-1"
              :active="activeTab === 0 && selectedFeedId === sub.feedId"
              @click="selectFeed(sub.feedId)"
            />
          </v-list-group>
        </v-list-group>

        <!-- Other Items -->
        <v-tooltip
          v-for="(item, i) in navItems.slice(1)"
          :key="i + 1"
          :text="item.label"
          location="end"
          :disabled="drawerOpen"
        >
          <template #activator="{ props: tooltipProps }">
            <v-list-item
              v-bind="tooltipProps"
              :value="i + 1"
              :active="activeTab === i + 1"
              active-color="primary"
              rounded="xl"
              class="mb-1"
              :class="drawerOpen ? 'text-body-2 px-4' : 'justify-center px-1'"
              @click="navigateTo(i + 1)"
            >
              <template #prepend>
                <div :class="drawerOpen ? 'mr-4' : 'mx-auto'">
                  <v-icon :color="activeTab === i + 1 ? 'primary' : 'on-surface-variant'" size="24">
                    {{ activeTab === i + 1 ? item.activeIcon : item.icon }}
                  </v-icon>
                </div>
              </template>
              <v-list-item-title v-if="drawerOpen" :class="activeTab === i + 1 ? 'text-primary font-weight-bold' : 'text-on-surface-variant'">
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

      <!-- Desktop sidebar toggle (Now toggles Article List if on Articles page) -->
      <v-app-bar-nav-icon
        class="d-none d-md-flex ml-1"
        @click="activeTab === 0 ? (articleListOpen = !articleListOpen) : (drawerOpen = !drawerOpen)"
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
        <!-- User Menu -->
        <v-menu location="bottom end" transition="scale-transition">
          <template #activator="{ props }">
            <v-avatar color="primary" size="34" class="mr-3 cursor-pointer" v-bind="props">
              <span class="text-caption font-weight-bold text-on-primary">A</span>
            </v-avatar>
          </template>
          <v-list rounded="lg" class="mt-2" elevation="8">
            <v-list-item prepend-icon="mdi-logout" title="退出登录" @click="logout" color="error" />
          </v-list>
        </v-menu>
      </template>
    </v-app-bar>

    <!-- ========== Main Content ========== -->
    <v-main class="main-content">
      <v-container fluid class="pa-4 pa-sm-6 pa-md-8 max-content-width">
        <transition name="fade-slide" mode="out-in">
          <component 
            :is="currentView" 
            :key="activeTab" 
            :feed-id="selectedFeedId" 
            :is-read="filterRead"
            :is-starred="filterStarred"
            v-model:is-sidebar-visible="articleListOpen"
          />
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
