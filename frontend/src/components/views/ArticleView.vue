<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  mdiMagnify,
  mdiCheckCircle,
  mdiCircleOutline,
  mdiStar,
  mdiTextSearch,
  mdiTranslate,
  mdiTextBoxSearchOutline,
  mdiStarOutline,
  mdiOpenInNew,
  mdiAccountOutline,
  mdiClockOutline,
  mdiNewspaperVariantOutline,
  mdiChevronUp,
  mdiChevronDown,
  mdiArrowLeft,
} from '@mdi/js'
import ArticleContent from '../ArticleContent.vue'
import { apiFetch } from '../../utils/api'
import { useDisplay } from 'vuetify'

const { t } = useI18n()
// 窄屏下文章列表与正文不能并排：一次只显示一个面板。
const { mdAndUp } = useDisplay()
const mobileShowContent = ref(false)

const props = defineProps<{
  feedId?: number,
  isRead?: boolean,
  isStarred?: boolean
}>()

const articles = ref<any[]>([])
const loading = ref(false)
const selectedArticle = ref<any>(null)
const articleDetail = ref<any>(null)
const blocks = ref<any[]>([])
const stitchedContent = ref('')
const articleSearch = ref('')
const contentArea = ref<HTMLElement | null>(null)
const customTransStyle = ref('')

const filteredArticles = computed(() => {
  if (!articleSearch.value.trim()) return articles.value
  const q = articleSearch.value.trim().toLowerCase()
  return articles.value.filter(a =>
    (a.title && a.title.toLowerCase().includes(q)) ||
    (a.translatedTitle && a.translatedTitle.toLowerCase().includes(q)) ||
    (a.feedTitle && a.feedTitle.toLowerCase().includes(q))
  )
})

let articlesRequestSeq = 0

const fetchArticles = async () => {
  const seq = ++articlesRequestSeq
  loading.value = true
  try {
    const params = new URLSearchParams()
    if (props.feedId) params.append('feed_id', props.feedId.toString())
    if (props.isRead !== undefined) params.append('is_read', props.isRead.toString())
    if (props.isStarred !== undefined) params.append('is_starred', props.isStarred.toString())

    const res = await apiFetch('/api/articles?' + params.toString())
    const data = await res.json()
    // 快速切源/搜索时可能先后发出多个请求，只让最后一次落地
    if (seq !== articlesRequestSeq) return
    articles.value = data
    // 列表整体替换对象，选中项要重新指向新副本，否则已读/星标状态会各走各的
    const current = selectedArticle.value
    if (current) {
      const fresh = data.find((a: any) => a.id === current.id)
      if (fresh) selectedArticle.value = fresh
    }
  } finally {
    if (seq === articlesRequestSeq) loading.value = false
  }
}

const fetchArticleDetail = async (id: number) => {
  const res = await apiFetch(`/api/articles/${id}`)
  const data = await res.json()
  articleDetail.value = data.detail
  blocks.value = data.blocks
  stitchedContent.value = data.content
  
  if (!articleDetail.value.isRead) {
    updateReadStatus(id, true)
  }
}

const updateReadStatus = async (id: number, read: boolean) => {
  await apiFetch(`/api/articles/${id}/read`, {
    method: 'POST',
    headers: { 
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ read })
  })
  if (articleDetail.value && articleDetail.value.id === id) {
    articleDetail.value.isRead = read
  }
  const art = articles.value.find(a => a.id === id)
  if (art) art.isRead = read
}

const toggleStar = async (id: number, current: boolean) => {
  await apiFetch(`/api/articles/${id}/star`, {
    method: 'POST',
    headers: { 
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ starred: !current })
  })
  if (articleDetail.value && articleDetail.value.id === id) {
    articleDetail.value.isStarred = !current
  }
  const art = articles.value.find(a => a.id === id)
  if (art) art.isStarred = !current
}

const translateBtnLoading = ref(false)
const summarizeBtnLoading = ref(false)
const snackbar = ref({ show: false, text: '', color: '' })

const isTranslated = computed(() => {
  // 修改逻辑：只有当序号大于等于 0 的正文块也有翻译时，才认为“全文已翻”，从而禁用按钮
  return blocks.value.length > 0 && blocks.value.some(b => b.blockIndex >= 0 && b.transText && b.transText.trim().length > 0)
})

const hasSummary = computed(() => {
  return articleDetail.value && articleDetail.value.summary && articleDetail.value.summary.trim().length > 0
})

const translateArticle = async (id: number) => {
  translateBtnLoading.value = true
  try {
    const res = await apiFetch(`/api/articles/${id}/translate`, {
      method: 'POST'
    })
    if (res.ok) {
      await fetchArticleDetail(id)
      snackbar.value = { show: true, text: t('article.translate_started'), color: 'info' }
    } else {
       alert(t('article.ai_translate') + ' failed: ' + await res.text())
    }
  } finally {
    translateBtnLoading.value = false
  }
}

const summarizeArticle = async (id: number) => {
  summarizeBtnLoading.value = true
  try {
    const res = await apiFetch(`/api/articles/${id}/summarize`, {
      method: 'POST'
    })
    if (res.ok) {
      await fetchArticleDetail(id)
    } else {
       alert(t('article.ai_summary') + ' failed: ' + await res.text())
    }
  } finally {
    summarizeBtnLoading.value = false
  }
}

const fetchUserSettings = async () => {
    try {
        const res = await apiFetch('/api/user/setting')
        if (res.ok) {
            const data = await res.json()
            customTransStyle.value = data.custom_trans_style || ''
        }
    } catch (e) {
        console.error('Failed to fetch user settings', e)
    }
}

let eventsSource: EventSource | null = null
let articlesRefreshTimer: ReturnType<typeof setTimeout> | null = null

// 后台同步/翻译每完成一次就推一条 SSE。逐条全量重拉列表会造成刷新风暴，
// 这里按 400ms 合并（与 QueueView 一致），避免列表在同步期间反复重建。
const scheduleArticlesRefresh = () => {
  if (articlesRefreshTimer) clearTimeout(articlesRefreshTimer)
  articlesRefreshTimer = setTimeout(() => {
    articlesRefreshTimer = null
    fetchArticles()
  }, 400)
}

onMounted(() => {
  fetchArticles()
  fetchUserSettings()

  // 📡 开启实时推送监听
  eventsSource = new EventSource('/api/events')
  eventsSource.onmessage = (event) => {
    const msg = event.data
    // 信号 A: 订阅同步完成
    if (msg === 'REFRESH_FEEDS') {
      scheduleArticlesRefresh()
    }
    // 信号 B: 文章翻译/总结完成
    if (msg.startsWith('ARTICLE_UPDATED:')) {
      const updatedId = parseInt(msg.split(':')[1])
      if (selectedArticle.value && selectedArticle.value.id === updatedId) {
        fetchArticleDetail(updatedId)
      }
      scheduleArticlesRefresh() // 合并刷新列表标记
    }
  }
})

onUnmounted(() => {
  if (articlesRefreshTimer) clearTimeout(articlesRefreshTimer)
  articlesRefreshTimer = null
  eventsSource?.close()
  eventsSource = null
})

watch(() => [props.feedId, props.isRead, props.isStarred], () => {
  articleSearch.value = ''
  fetchArticles()
})

const isSidebarVisible = defineModel<boolean>('isSidebarVisible', { default: true })

const selectArticle = (article: any) => {
  selectedArticle.value = article
  fetchArticleDetail(article.id)
  isSidebarVisible.value = false // 点击文章后收起列表
  mobileShowContent.value = true // 窄屏切换到正文面板
}

// 窄屏返回列表：同时恢复被 selectArticle 收起的列表面板。
const mobileBack = () => {
  mobileShowContent.value = false
  isSidebarVisible.value = true
}

const formatDate = (ts: number) => {
  return new Date(ts * 1000).toLocaleString()
}

const canPrev = computed(() => {
  const index = filteredArticles.value.findIndex(a => a.id === selectedArticle.value?.id)
  return index > 0
})

const canNext = computed(() => {
  const index = filteredArticles.value.findIndex(a => a.id === selectedArticle.value?.id)
  return index !== -1 && index < filteredArticles.value.length - 1
})

const prevArticle = () => {
  const index = filteredArticles.value.findIndex(a => a.id === selectedArticle.value?.id)
  if (index > 0) {
    selectArticle(filteredArticles.value[index - 1])
  }
}

const nextArticle = () => {
  const index = filteredArticles.value.findIndex(a => a.id === selectedArticle.value?.id)
  if (index !== -1 && index < filteredArticles.value.length - 1) {
    selectArticle(filteredArticles.value[index + 1])
  }
}

// 只在真正换了文章时回到顶部：后台刷新会把 selectedArticle 换成新副本，
// 按对象 watch 会让正在读的文章被拉回开头。
watch(() => selectedArticle.value?.id, () => {
  if (contentArea.value) {
    contentArea.value.scrollTo({ top: 0, behavior: 'smooth' })
  }
})
</script>

<template>
  <div class="article-view d-flex h-100 overflow-hidden">
    <!-- Article List -->
    <div 
      v-show="mdAndUp || !mobileShowContent"
      class="article-list border-e transition-sidebar" 
    >
      <!-- 搜索栏 -->
      <div class="article-search-box">
        <v-text-field
          v-model="articleSearch"
          :placeholder="$t('article.search')"
          variant="outlined"
          density="compact"
          rounded="xl"
          hide-details
          clearable
          bg-color="surface"
          color="primary"
        >
          <template #prepend-inner>
            <v-icon size="18" color="medium-emphasis">{{ mdiMagnify }}</v-icon>
          </template>
        </v-text-field>
      </div>

      <v-list 
        lines="two" 
        class="pa-0 article-list-scroll custom-scrollbar"
      >
        <v-list-item
          v-for="article in filteredArticles"
          :key="article.id"
          @click="selectArticle(article)"
          :active="selectedArticle?.id === article.id"
          class="px-4 py-3"
        >
          <div class="d-flex flex-column gap-1">
            <div class="d-flex align-center gap-2">
              <v-icon
                :color="article.isRead ? 'success' : 'primary'"
                size="12"
                class="mr-1"
                :icon="article.isRead ? mdiCheckCircle : mdiCircleOutline"
              />
              <span class="text-caption text-medium-emphasis">{{ article.feedTitle }}</span>
              <v-spacer />
              <v-icon v-if="article.isStarred" size="x-small" color="warning">{{ mdiStar }}</v-icon>
            </div>
            <div class="text-subtitle-2 font-weight-bold line-clamp-2" :class="article.isRead ? 'text-medium-emphasis' : ''">
              {{ article.translatedTitle || article.title }}
            </div>
            <div class="text-caption text-medium-emphasis mt-1">
              {{ article.publishedAt ? formatDate(article.publishedAt) : '' }}
            </div>
          </div>
        </v-list-item>

        <!-- 无搜索结果提示 -->
        <div
          v-if="!loading && articleSearch && filteredArticles.length === 0"
          class="d-flex flex-column align-center justify-center pa-8 text-medium-emphasis"
        >
          <v-icon size="36" class="mb-3" color="medium-emphasis">{{ mdiTextSearch }}</v-icon>
          <span class="text-body-2">{{ $t('article.empty_search') }}</span>
          <v-btn
            variant="text"
            color="primary"
            size="small"
            class="mt-2 text-none"
            @click="articleSearch = ''"
          >{{ $t('article.clear_search') }}</v-btn>
        </div>

        <div v-if="loading" class="d-flex justify-center pa-4">
          <v-progress-circular indeterminate size="24" width="2" color="primary" />
        </div>
      </v-list>
    </div>

    <!-- Article Content -->
    <div 
        v-show="mdAndUp || mobileShowContent"
        ref="contentArea"
        class="article-content flex-grow-1 overflow-y-auto px-6 py-8 custom-scrollbar bg-surface position-relative"
    >
      <v-btn
        v-if="!mdAndUp"
        variant="text"
        class="text-none mb-2"
        :prepend-icon="mdiArrowLeft"
        @click="mobileBack"
      >
        {{ $t('article.back_to_list') }}
      </v-btn>

      <div v-if="selectedArticle && articleDetail" class="mx-auto" style="max-width: 900px;">
        <div class="article-head d-flex align-start justify-space-between mb-2">
            <h1 class="article-title text-h4 font-weight-bold flex-grow-1 mr-4">{{ articleDetail.title }}</h1>
            <div class="article-actions d-flex">
                <v-btn
                    icon
                    variant="text"
                    :color="isTranslated ? 'success' : 'grey'"
                    @click="translateArticle(articleDetail.id)"
                    :loading="translateBtnLoading"
                    :disabled="isTranslated"
                    :title="$t('article.ai_translate')"
                    :class="{ 'btn-pulse': !isTranslated && !translateBtnLoading }"
                >
                    <v-icon>{{ mdiTranslate }}</v-icon>
                    <template v-slot:loader>
                        <div class="squiggle-loader">
                            <svg viewBox="0 0 100 100">
                                <path d="M50,10c2.3,0,4.6,3.4,6.9,6.9s4.6,6.9,6.9,6.9s4.6-3.4,6.9-6.9s4.6-6.9,6.9-6.9c2.3,0,4.6,3.4,6.9,6.9s4.6,6.9,6.9,6.9s4.6-3.4,6.9-6.9S97.7,10,100,10" transform="translate(0,0)"/>
                                <circle cx="50" cy="50" r="40" class="squiggle-path" />
                            </svg>
                        </div>
                    </template>
                </v-btn>
                <v-btn
                    icon
                    variant="text"
                    :color="hasSummary ? 'success' : 'grey'"
                    @click="summarizeArticle(articleDetail.id)"
                    :loading="summarizeBtnLoading"
                    :disabled="hasSummary"
                    :title="$t('article.ai_summary')"
                    :class="{ 'btn-pulse': !hasSummary && !summarizeBtnLoading }"
                >
                    <v-icon>{{ mdiTextBoxSearchOutline }}</v-icon>
                    <template v-slot:loader>
                        <div class="squiggle-loader">
                            <svg viewBox="0 0 100 100">
                                <circle cx="50" cy="50" r="38" class="squiggle-path" />
                            </svg>
                        </div>
                    </template>
                </v-btn>
                <v-btn
                    icon
                    variant="text"
                    :color="articleDetail.isStarred ? 'warning' : ''"
                    @click="toggleStar(articleDetail.id, articleDetail.isStarred)"
                    :title="$t('article.star')"
                >
                    <v-icon>{{ articleDetail.isStarred ? mdiStar : mdiStarOutline }}</v-icon>
                </v-btn>
                <v-btn
                    icon
                    variant="text"
                    :color="articleDetail.isRead ? 'success' : 'primary'"
                    @click="updateReadStatus(articleDetail.id, !articleDetail.isRead)"
                    :title="articleDetail.isRead ? $t('article.mark_unread') : $t('article.mark_read')"
                >
                    <v-icon>{{ articleDetail.isRead ? mdiCheckCircle : mdiCircleOutline }}</v-icon>
                </v-btn>
                <v-btn
                    v-if="articleDetail.link"
                    variant="text"
                    :icon="mdiOpenInNew"
                    :href="articleDetail.link"
                    target="_blank"
                    :title="$t('article.open_browser')"
                />
                <!-- 窄屏没有悬浮按钮（会盖住正文），翻页放到标题操作区 -->
                <template v-if="!mdAndUp">
                  <v-btn
                      icon
                      variant="text"
                      :disabled="!canPrev"
                      @click="prevArticle"
                      :title="$t('article.prev_article')"
                  >
                      <v-icon>{{ mdiChevronUp }}</v-icon>
                  </v-btn>
                  <v-btn
                      icon
                      variant="text"
                      :disabled="!canNext"
                      @click="nextArticle"
                      :title="$t('article.next_article')"
                  >
                      <v-icon>{{ mdiChevronDown }}</v-icon>
                  </v-btn>
                </template>
            </div>
        </div>
        
        <div class="d-flex align-center mb-12 text-medium-emphasis py-6 border-y" style="border-color: rgba(var(--v-border-color), 0.08) !important;">
          <span v-if="articleDetail.author" class="d-flex align-center mr-12">
            <v-icon size="small" class="mr-3">{{ mdiAccountOutline }}</v-icon>
            {{ articleDetail.author }}
          </span>
          <span class="d-flex align-center">
            <v-icon size="small" class="mr-3">{{ mdiClockOutline }}</v-icon>
            {{ articleDetail.publishedAt ? formatDate(articleDetail.publishedAt) : '' }}
          </span>
        </div>

        <ArticleContent :content="stitchedContent" :custom-style="customTransStyle" />

        <!-- 悬浮导航按钮 -->
        <div v-if="mdAndUp" class="floating-nav d-flex flex-column ga-6">
          <v-btn
            icon
            elevation="4"
            color="surface"
            :disabled="!canPrev"
            @click="prevArticle"
            size="large"
            class="nav-btn"
            :title="$t('common.prev')"
          >
            <v-icon color="primary">{{ mdiChevronUp }}</v-icon>
          </v-btn>
          <v-btn
            icon
            elevation="4"
            color="primary"
            :disabled="!canNext"
            @click="nextArticle"
            size="large"
            class="nav-btn scale-up"
            :title="$t('common.next')"
          >
            <v-icon>{{ mdiChevronDown }}</v-icon>
          </v-btn>
        </div>
      </div>
      <div v-else class="h-100 d-flex flex-column align-center justify-center text-medium-emphasis opacity-60">
        <v-icon size="64" class="mb-4">{{ mdiNewspaperVariantOutline }}</v-icon>
        <p>{{ $t('article.empty_state') }}</p>
      </div>
    </div>

    <!-- 提示消息 -->
    <v-snackbar v-model="snackbar.show" :color="snackbar.color" rounded="pill" elevation="12">
      {{ snackbar.text }}
      <template v-slot:actions>
        <v-btn variant="text" @click="snackbar.show = false">{{ $t('common.confirm') }}</v-btn>
      </template>
    </v-snackbar>
  </div>
</template>

<style scoped>
.article-view {
  height: calc(100vh - var(--top-header-height, 64px)); /* Exact height of viewport minus global app bar */
  margin: -16px -16px -32px; /* Pull out to edges of container padding from HomeView */
}

@media (min-width: 960px) {
  .article-view {
    margin: -32px -32px -32px;
  }
}

.transition-sidebar {
  transition: 
    width var(--md-motion-duration-long) var(--md-motion-easing-emphasized),
    opacity var(--md-motion-duration-medium) var(--md-motion-easing-emphasized),
    transform var(--md-motion-duration-long) var(--md-motion-easing-emphasized);
  width: 300px;
  flex-shrink: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.sidebar-hidden {
  width: 0 !important;
  opacity: 0;
  border-right: none !important;
}

.article-search-box {
  flex-shrink: 0;
  padding: 12px 14px;
  background: rgb(var(--v-theme-surface));
  border-bottom: 1px solid rgba(var(--v-border-color), 0.12);
  height: var(--top-header-height, 64px) !important; 
  min-height: var(--top-header-height, 64px) !important;
  max-height: var(--top-header-height, 64px) !important;
  box-sizing: border-box !important;
  display: flex;
  align-items: center;
}

.article-list-scroll .v-list-item {
  transition: transform var(--md-motion-duration-short) var(--md-motion-easing-emphasized) !important;
}

.article-list-scroll .v-list-item:active {
  transform: scale(0.96); /* MD3 Click Spring */
}

.article-list-scroll {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(var(--v-border-color), 0.1);
  border-radius: 10px;
}
.custom-scrollbar:hover::-webkit-scrollbar-thumb {
  background: rgba(var(--v-border-color), 0.2);
}

/* AI 按钮心跳动画 */
.btn-pulse {
  animation: pulse-animation 2.5s infinite;
}

@keyframes pulse-animation {
  0% { transform: scale(1); opacity: 0.8; }
  50% { transform: scale(1.1); opacity: 1; filter: drop-shadow(0 0 5px rgba(var(--v-theme-primary), 0.3)); }
  100% { transform: scale(1); opacity: 0.8; }
}

@keyframes circular-pseudo-rotate {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

/* Material You Squiggle 动画 (折线/波浪模式) */
.squiggle-loader {
  width: 42px;
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.squiggle-path {
  fill: none;
  stroke: currentColor;
  stroke-width: 4;
  stroke-linecap: round;
  /* 使用虚线模拟折线/波浪的视觉感 */
  stroke-dasharray: 15, 12;
  animation: squiggle-rotate 2s linear infinite, squiggle-dash 1.5s ease-in-out infinite alternate;
  filter: drop-shadow(0 0 2px rgba(var(--v-theme-primary), 0.2));
}

@keyframes squiggle-rotate {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes squiggle-dash {
  0% {
    stroke-dashoffset: 0;
    opacity: 0.7;
  }
  100% {
    stroke-dashoffset: 50;
    opacity: 1;
    stroke-width: 5;
  }
}

/* 营造折线感的特殊滤镜（可选，增强波纹感） */
.squiggle-loader svg {
  overflow: visible;
  transform-origin: center;
  transition: all 0.3s ease;
}

.floating-nav {
  position: fixed;
  bottom: 40px;
  right: 40px;
  z-index: 100;
}

.nav-btn {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) !important;
}

.nav-btn:hover {
  transform: translateY(-5px);
}

.nav-btn:active {
  transform: scale(0.9);
}

.scale-up {
    border: 2px solid rgba(var(--v-theme-primary), 0.1);
}

/* 窄屏：容器 padding 为 pa-4 (16px)，负边距必须同步，否则右侧溢出 8px。 */
@media (min-width: 600px) and (max-width: 959px) {
  .article-view {
    margin: -24px -24px -32px;
  }
}

/* 窄屏一次只显示一个面板：列表占满宽度，正文由 v-show 控制。 */
@media (max-width: 959px) {
  .article-list {
    width: 100%;
  }

  .article-content {
    padding-inline: 16px;
  }

  /* 底部导航（56px）是浮在内容之上的；v-main 已预留 padding-bottom，
     但本视图用 100vh 硬算高度并抵消了 padding，必须自己减掉导航高度，
     否则正文最后一段会被导航条盖住。 */
  .article-view {
    height: calc(100vh - var(--top-header-height, 64px) - 56px);
  }

  /* 上一页/下一页按钮同样要抬到导航条上方，否则后一个按钮被遮住。 */
  .floating-nav {
    bottom: calc(56px + 16px);
    right: 16px;
  }
}

/* 手机上标题与操作按钮并排会把标题压成 4-5 字一行，改为上下堆叠。 */
@media (max-width: 599px) {
  .article-head {
    flex-wrap: wrap;
  }

  .article-title {
    flex: 1 1 100%;
    margin-right: 0 !important;
    margin-bottom: 8px;
  }
}
</style>

