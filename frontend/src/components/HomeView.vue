<script setup lang="ts">
import { ref, computed, shallowRef } from 'vue'
import { useTheme, useDisplay } from 'vuetify'
import { useI18n } from 'vue-i18n'
import SettingsView from './views/SettingsView.vue'
import ApiView from './views/ApiView.vue'
import SubscriptionView from './views/SubscriptionView.vue'
import QueueView from './views/QueueView.vue'
import ArticleView from './views/ArticleView.vue'
import { onMounted } from 'vue'
import { 
  mdiNewspaperVariant, 
  mdiNewspaperVariantOutline, 
  mdiCircleMedium, 
  mdiStarOutline, 
  mdiAccountCircleOutline, 
  mdiChevronUp, 
  mdiChevronDown, 
  mdiTranslate, 
  mdiWeatherSunny, 
  mdiWeatherNight, 
  mdiLogout,
  mdiRss,
  mdiRssBox,
  mdiTextBoxOutline,
  mdiTextBox,
  mdiKeyVariant,
  mdiCog,
  mdiCogOutline
} from '@mdi/js'
import { apiFetch } from '../utils/api'

const theme = useTheme()
const { mdAndUp } = useDisplay()
const isDark = ref(theme.global.current.value.dark)

// Computed drawer offset for app bar alignment on desktop
const drawerOffset = computed(() => {
  if (!mdAndUp.value) return '0px'
  return drawerOpen.value ? '240px' : '88px'
})
const activeTab = ref(0)
const activeJobsCount = ref(0)
const mobileDrawer = ref(false)
const drawerOpen = ref(true)
const articlesExpanded = ref(true) // manual expand state for articles group
const openedGroups = ref(['articles'])
const collapsedCategories = ref<string[]>([]) // track collapsed custom categories

const toggleCategory = (cat: string) => {
  if (collapsedCategories.value.includes(cat)) {
    collapsedCategories.value = collapsedCategories.value.filter(c => c !== cat)
  } else {
    collapsedCategories.value.push(cat)
  }
}

const toggleTheme = () => {
  isDark.value = !isDark.value
  theme.global.name.value = isDark.value ? 'dark' : 'light'
}

const { t, locale } = useI18n()
const setLanguage = (lang: string) => {
  locale.value = lang
  localStorage.setItem('locale', lang)
}

const navItemsLocalized = computed(() => {
  const badgeCount = activeJobsCount.value
  return [
    { key: 'articles', icon: mdiNewspaperVariantOutline, activeIcon: mdiNewspaperVariant, label: t('nav.articles') },
    { key: 'subscriptions', icon: mdiRss,                  activeIcon: mdiRssBox,         label: t('nav.subscriptions') },
    { key: 'queue', icon: mdiTextBoxOutline,     activeIcon: mdiTextBox,          label: t('nav.queue'), badge: badgeCount > 0 ? badgeCount : undefined },
    { key: 'api_keys', icon: mdiKeyVariant,          activeIcon: mdiKeyVariant,     label: t('nav.api_keys') },
    { key: 'settings', icon: mdiCogOutline,          activeIcon: mdiCog,             label: t('nav.settings') },
  ]
})

const views = shallowRef([ArticleView, SubscriptionView, QueueView, ApiView, SettingsView])
const currentView = computed(() => views.value[activeTab.value] || ArticleView)

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
    const response = await apiFetch('/api/feeds')
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

const unreadCount = computed(() => subscriptions.value.reduce((acc, sub) => acc + sub.unreadCount, 0))
const starredCount = computed(() => subscriptions.value.reduce((acc, sub) => acc + sub.starredCount, 0))

const groupedSubscriptions = computed(() => {
  const groups: Record<string, any[]> = {}
  subscriptions.value.forEach(sub => {
    const cat = (!sub.category || sub.category === '未分类') ? t('common.uncategorized') : sub.category
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
  // 使用 reload 是最稳妥的退出方式，确保所有内存状态清空
  window.location.reload()
}

const fetchActiveJobsCount = async () => {
  try {
    const response = await apiFetch('/api/jobs')
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
const getCategoryUnreadCount = (subs: any[]) => {
  return subs.reduce((acc, sub) => acc + (sub.unreadCount || 0), 0)
}
</script>

<template>
  <v-layout>
    <!-- ========== Mobile Navigation Drawer ========== -->
    <v-navigation-drawer
      v-if="!mdAndUp"
      v-model="mobileDrawer"
      temporary
      location="left"
      width="280"
      color="surface"
    >
      <!-- Logo -->
      <div class="pa-6 pb-4">
        <div class="d-flex align-center gap-3">
          <v-avatar class="logo-avatar" size="44" rounded="lg">
            <img src="/favicon.svg?v=latest" alt="logo" style="width: 100%; height: 100%; object-fit: cover;" />
          </v-avatar>
          <div>
            <p class="text-h6 font-weight-bold" style="line-height:1.2; font-family: 'Noto Serif SC', serif !important;">TranRSS</p>
            <p class="text-caption text-medium-emphasis">{{ $t('nav.expert') }}</p>
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
              color="primary"
              rounded="xl"
              class="mb-1 text-body-2"
              @click="navigateTo(0); selectFeed(undefined, undefined, undefined)"
            >
              <template #prepend>
                <v-icon :color="activeTab === 0 ? 'primary' : 'on-surface-variant'" :icon="activeTab === 0 ? mdiNewspaperVariant : mdiNewspaperVariantOutline" />
              </template>
              <v-list-item-title :class="activeTab === 0 ? 'text-primary' : 'text-on-surface-variant'">
                {{ $t('nav.all_articles') }}
              </v-list-item-title>
            </v-list-item>
          </template>

          <v-list-item 
            :prepend-icon="mdiCircleMedium" 
            density="compact" 
            class="pl-6 mb-1 text-body-2" 
            :active="activeTab === 0 && filterRead === false" 
            @click="selectFeed(undefined, false)" 
          >
            <v-list-item-title>{{ $t('nav.unread') }}</v-list-item-title>
            <template #append v-if="unreadCount > 0">
              <v-badge :content="unreadCount" color="primary" inline />
            </template>
          </v-list-item>
          <v-list-item 
            :prepend-icon="mdiStarOutline" 
            density="compact" 
            class="pl-6 mb-1 text-body-2" 
            :active="activeTab === 0 && filterStarred === true" 
            @click="selectFeed(undefined, undefined, true)" 
          >
            <v-list-item-title>{{ $t('nav.starred') }}</v-list-item-title>
            <template #append v-if="starredCount > 0">
              <v-badge :content="starredCount" color="primary" inline />
            </template>
          </v-list-item>

          <v-list-group v-for="(subs, cat) in groupedSubscriptions" :key="cat" :value="cat">
            <template #activator="{ props }">
              <v-list-item 
                v-bind="props" 
                :title="cat as string" 
                density="compact" 
                class="text-body-2"
              >
                <template #append v-if="getCategoryUnreadCount(subs) > 0">
                  <v-badge :content="getCategoryUnreadCount(subs)" color="primary" inline />
                </template>
              </v-list-item>
            </template>
            <v-list-item
              v-for="sub in subs"
              :key="sub.id"
              :title="sub.title"
              density="compact"
              class="pl-10 text-caption rounded-lg mb-1"
              :active="activeTab === 0 && selectedFeedId === sub.feedId"
              @click="selectFeed(sub.feedId)"
            >
              <template #append v-if="sub.unreadCount > 0">
                <span class="text-caption text-medium-emphasis">{{ sub.unreadCount }}</span>
              </template>
            </v-list-item>
          </v-list-group>
        </v-list-group>

        <!-- Other Items -->
        <v-list-item
          v-for="(item, i) in navItemsLocalized.slice(1)"
          :key="i + 1"
          :active="activeTab === i + 1"
          color="primary"
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
            :prepend-icon="mdiAccountCircleOutline"
            :title="$t('nav.admin')"
            subtitle="admin@tranrss.app"
            rounded="xl"
            class="text-body-2"
          />
        </div>
      </template>
    </v-navigation-drawer>

    <!-- ========== Desktop Sidebar ========== -->
    <v-navigation-drawer
      v-if="mdAndUp"
      :model-value="true"
      permanent
      app
      :rail="!drawerOpen"
      rail-width="88"
      width="240"
      color="surface"
      class="border-e custom-sidebar"
    >
      <!-- Logo container: matched height to top bar (64px) for perfect divider alignment -->
      <div class="px-5 d-flex align-center" style="height: 64px;">
        <v-avatar class="logo-avatar cursor-pointer" size="40" rounded="lg" @click="drawerOpen = !drawerOpen">
          <img src="/favicon.svg?v=latest" alt="logo" style="width: 100%; height: 100%; object-fit: cover;" />
        </v-avatar>
        <div v-if="drawerOpen" class="ml-3">
          <p class="text-h6 font-weight-bold mb-0" style="line-height:1; font-family: 'Noto Serif SC', serif !important;">TranRSS</p>
        </div>
      </div>

      <v-divider class="mx-3 mb-1" />

      <!-- Nav Items -->
      <v-list nav :class="[drawerOpen ? 'pa-3' : 'pa-1', 'custom-sidebar-list']" v-model:opened="openedGroups">
        <!-- Articles Group -->
        <div class="articles-group-container">
          <!-- Group header: using v-list-item for consistency with other items -->
          <v-list-item
            v-if="drawerOpen"
            rounded="xl"
            class="mb-1"
            :class="activeTab === 0 && !selectedFeedId && filterRead === undefined && filterStarred === undefined ? 'px-4' : 'px-4'"
            :active="activeTab === 0 && !selectedFeedId && filterRead === undefined && filterStarred === undefined"
            color="primary"
            @click="navigateTo(0); selectFeed(undefined, undefined, undefined)"
          >
            <template #prepend>
              <div class="mr-4">
                <v-icon :color="activeTab === 0 ? 'primary' : 'on-surface-variant'" size="24" :icon="activeTab === 0 ? mdiNewspaperVariant : mdiNewspaperVariantOutline" />
              </div>
            </template>
            <v-list-item-title :class="activeTab === 0 ? 'text-primary font-weight-bold' : 'text-on-surface-variant'">
              {{ $t('nav.all_articles') }}
            </v-list-item-title>
            <template #append>
              <v-btn icon variant="text" size="small" density="compact" @click.stop="articlesExpanded = !articlesExpanded">
                <v-icon size="18" color="on-surface-variant" :icon="articlesExpanded ? mdiChevronUp : mdiChevronDown" />
              </v-btn>
            </template>
          </v-list-item>

          <!-- Rail mode version: centered to match other items -->
          <v-list-item
            v-else
            :active="activeTab === 0 && !selectedFeedId && filterRead === undefined && filterStarred === undefined"
            color="primary"
            rounded="xl"
            class="mb-1 justify-center px-1"
            @click="navigateTo(0); selectFeed(undefined, undefined, undefined)"
          >
            <template #prepend>
              <div class="mx-auto">
                <v-icon :color="activeTab === 0 ? 'primary' : 'on-surface-variant'" size="24" :icon="activeTab === 0 ? mdiNewspaperVariant : mdiNewspaperVariantOutline" />
              </div>
            </template>
          </v-list-item>

          <!-- Sub-items (shown only when expanded AND articlesExpanded is true) -->
          <template v-if="drawerOpen && articlesExpanded">
            <v-list-item 
              :prepend-icon="mdiCircleMedium"              density="compact" 
              class="pl-10 mb-1 text-body-2 rounded-lg" 
              :active="activeTab === 0 && filterRead === false" 
              @click="selectFeed(undefined, false)" 
            >
              <v-list-item-title>{{ $t('nav.unread') }}</v-list-item-title>
              <template #append v-if="unreadCount > 0">
                <v-badge :content="unreadCount" color="primary" inline />
              </template>
            </v-list-item>
            <v-list-item 
              :prepend-icon="mdiStarOutline"              density="compact" 
              class="pl-10 mb-1 text-body-2 rounded-lg" 
              :active="activeTab === 0 && filterStarred === true" 
              @click="selectFeed(undefined, undefined, true)" 
            >
              <v-list-item-title>{{ $t('nav.starred') }}</v-list-item-title>
              <template #append v-if="starredCount > 0">
                <v-badge :content="starredCount" color="primary" inline />
              </template>
            </v-list-item>

            <template v-for="(subs, cat) in groupedSubscriptions" :key="cat">
              <!-- Custom Category Header with Expand/Collapse -->
              <v-list-item 
                density="compact" 
                class="text-body-2 pl-10 cursor-pointer rounded-lg mb-1" 
                @click="toggleCategory(cat as string)"
              >
                <template #prepend>
                   <v-icon size="20" class="mr-2" color="on-surface-variant">
                     {{ collapsedCategories.includes(cat as string) ? mdiChevronDown : mdiChevronUp }}
                   </v-icon>
                </template>
                <v-list-item-title :class="collapsedCategories.includes(cat as string) ? 'text-on-surface-variant' : 'text-primary font-weight-medium'">
                  {{ cat }}
                </v-list-item-title>
                <template #append v-if="getCategoryUnreadCount(subs) > 0">
                  <v-badge :content="getCategoryUnreadCount(subs)" color="primary" inline />
                </template>
              </v-list-item>
              
              <!-- Subscriptions list in the category -->
              <template v-if="!collapsedCategories.includes(cat as string)">
                <v-list-item
                  v-for="sub in subs"
                  :key="sub.id"
                  :title="sub.title"
                  density="compact"
                  class="pl-14 text-caption rounded-lg mb-1"
                  :active="activeTab === 0 && selectedFeedId === sub.feedId"
                  @click="selectFeed(sub.feedId)"
                >
                  <template #append v-if="sub.unreadCount > 0">
                    <span class="text-caption text-medium-emphasis">{{ sub.unreadCount }}</span>
                  </template>
                </v-list-item>
              </template>
            </template>
          </template>
        </div>


        <!-- Other Items -->
        <v-list-item
          v-for="(item, i) in navItemsLocalized.slice(1)"
          :key="i + 1"
          :value="i + 1"
          :active="activeTab === i + 1"
          color="primary"
          rounded="xl"
          class="mb-1"
          :class="drawerOpen ? 'text-body-2 px-4' : 'justify-center px-1'"
          @click="navigateTo(i + 1)"
        >
          <template #prepend>
            <div :class="drawerOpen ? 'mr-4' : 'mx-auto'">
              <v-icon :color="activeTab === i + 1 ? 'primary' : 'on-surface-variant'" size="24" :icon="activeTab === i + 1 ? item.activeIcon : item.icon" />
            </div>
          </template>
          <v-list-item-title v-if="drawerOpen" :class="activeTab === i + 1 ? 'text-primary font-weight-bold' : 'text-on-surface-variant'">
            {{ item.label }}
          </v-list-item-title>
          <template v-if="item.badge && drawerOpen" #append>
            <v-badge :content="item.badge" color="primary" inline />
          </template>
        </v-list-item>
      </v-list>

      <template #append>
        <v-divider class="mx-3" />
        <div :class="drawerOpen ? 'pa-3' : 'pa-2 d-flex justify-center'">
          <v-list-item
            v-if="drawerOpen"
            :prepend-icon="mdiAccountCircleOutline"
            :title="$t('nav.admin')"
            subtitle="admin@tranrss.app"
            rounded="xl"
            class="text-body-2"
          />
          <v-avatar v-else color="surface-variant" size="38" rounded="lg">
            <v-icon size="20">{{ mdiAccountCircleOutline }}</v-icon>
          </v-avatar>
        </div>
      </template>
    </v-navigation-drawer>

    <!-- ========== Top App Bar ========== -->
    <v-app-bar
      app
      flat
      color="surface"
      class="border-b"
      height="64"
      :style="mdAndUp ? { 
        left: drawerOffset, 
        width: `calc(100% - ${drawerOffset})`,
        paddingLeft: '0px'
      } : {}"
    >

      <!-- Mobile menu button -->
      <v-app-bar-nav-icon
        v-if="!mdAndUp"
        class="ml-1"
        @click="mobileDrawer = !mobileDrawer"
      />

      <!-- Article list sidebar toggle: Only visible when on Articles tab (All Articles) -->
      <v-app-bar-nav-icon
        v-if="mdAndUp && activeTab === 0"
        @click="articleListOpen = !articleListOpen"
      />

      <!-- Spacer (Removed breadcrumb title articles as per user request) -->
      <v-spacer />

      <template #append>
        <!-- Compatible Language Select (Native fallback due to Vuetify 4.0 beta bugs) -->
        <div class="lang-select-wrapper mr-3">
          <v-icon size="small" class="lang-icon">{{ mdiTranslate }}</v-icon>
          <select 
            v-model="locale" 
            class="lang-native-select"
            @change="setLanguage(locale)"
          >
            <option value="zh">简体中文</option>
            <option value="en">English</option>
          </select>
          <v-icon size="small" class="lang-arrow">{{ mdiChevronDown }}</v-icon>
        </div>
        <!-- Theme toggle -->
        <v-btn icon variant="text" class="mr-1" @click="toggleTheme">
          <v-icon :icon="isDark ? mdiWeatherSunny : mdiWeatherNight" />
        </v-btn>
        <!-- User Menu -->
        <v-menu location="bottom end" transition="scale-transition">
          <template #activator="{ props }">
            <v-avatar color="primary" size="34" class="mr-3 cursor-pointer" v-bind="props">
              <span class="text-caption font-weight-bold text-on-primary">A</span>
            </v-avatar>
          </template>
          <v-list rounded="lg" class="mt-2" elevation="8">
            <v-list-item :prepend-icon="mdiLogout" :title="$t('nav.logout')" @click="logout" color="error" />
          </v-list>
        </v-menu>
      </template>
    </v-app-bar>

    <!-- ========== Main Content ========== -->
    <v-main class="main-content" style="height: 100vh; overflow-y: auto;">
      <v-container fluid class="pa-4 pa-sm-6 pa-md-8 pb-16 max-content-width">
        <component 
          :is="currentView" 
          :key="activeTab" 
          :feed-id="selectedFeedId" 
          :is-read="filterRead"
          :is-starred="filterStarred"
          v-model:is-sidebar-visible="articleListOpen"
        />
      </v-container>
    </v-main>

    <!-- ========== Mobile Bottom Navigation ========== -->
    <v-bottom-navigation
      v-if="!mdAndUp"
      v-model="activeTab"
      color="primary"
      bg-color="surface"
      elevation="8"
      grow
    >
      <v-btn v-for="(item, i) in navItemsLocalized" :key="i" :value="i" class="text-none">
        <v-badge
          v-if="item.badge"
          :content="item.badge"
          color="error"
          floating
          offset-x="-4"
          offset-y="4"
        >
          <v-icon :color="activeTab === i ? 'primary' : 'on-surface-variant'" :icon="activeTab === i ? item.activeIcon : item.icon" />
        </v-badge>
        <v-icon v-else :color="activeTab === i ? 'primary' : 'on-surface-variant'" :icon="activeTab === i ? item.activeIcon : item.icon" />
        <span class="text-caption mt-1" :class="activeTab === i ? 'text-primary' : 'text-on-surface-variant'">{{ item.label }}</span>
      </v-btn>
    </v-bottom-navigation>
  </v-layout>
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
  width: 100%;
}

.cursor-pointer {
  cursor: pointer;
}

.gap-1 { gap: 4px; }
.gap-3 { gap: 12px; }

/* Articles group header - two clickable zones */
.articles-group-header {
  display: flex;
  align-items: center;
  height: 40px;
  padding: 0 8px 0 16px;
  cursor: pointer;
  transition: background-color 0.15s ease;
  position: relative;
}
.articles-group-header:hover {
  background-color: rgba(var(--v-theme-on-surface), 0.06);
}
.articles-nav-zone {
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 0;
  height: 100%;
}
.articles-chevron-zone {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  flex-shrink: 0;
  transition: background-color 0.15s ease;
}
.articles-chevron-zone:hover {
  background-color: rgba(var(--v-theme-on-surface), 0.1);
}

.custom-sidebar :deep(.v-navigation-drawer__content) {
  display: flex !important;
  flex-direction: column !important;
}

.custom-sidebar-list {
  flex: 1 1 auto !important;
  overflow-y: auto !important;
}

/* Fixed alignment for rail icons to match logo position */
.custom-sidebar :deep(.v-list-item--active) .v-list-item__overlay {
  opacity: 0 !important;
}

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

<style scoped>
.lang-select-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  background: rgba(var(--v-theme-primary), 0.08);
  border-radius: 20px;
  padding: 0 12px;
  height: 32px;
  min-width: 130px;
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
  background: var(--v-theme-surface);
  color: var(--v-theme-on-surface);
}
</style>
