<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import ArticleContent from '../ArticleContent.vue'

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

const filteredArticles = computed(() => {
  if (!articleSearch.value.trim()) return articles.value
  const q = articleSearch.value.trim().toLowerCase()
  return articles.value.filter(a =>
    (a.title && a.title.toLowerCase().includes(q)) ||
    (a.feedTitle && a.feedTitle.toLowerCase().includes(q))
  )
})

const fetchArticles = async () => {
  loading.value = true
  try {
    const params = new URLSearchParams()
    if (props.feedId) params.append('feed_id', props.feedId.toString())
    if (props.isRead !== undefined) params.append('is_read', props.isRead.toString())
    if (props.isStarred !== undefined) params.append('is_starred', props.isStarred.toString())
    
    const res = await fetch('/api/articles?' + params.toString(), {
      headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
    })
    articles.value = await res.json()
  } finally {
    loading.value = false
  }
}

const fetchArticleDetail = async (id: number) => {
  const res = await fetch(`/api/articles/${id}`, {
    headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` }
  })
  const data = await res.json()
  articleDetail.value = data.detail
  blocks.value = data.blocks
  stitchedContent.value = data.content
  
  if (!articleDetail.value.isRead) {
    updateReadStatus(id, true)
  }
}

const updateReadStatus = async (id: number, read: boolean) => {
  await fetch(`/api/articles/${id}/read`, {
    method: 'POST',
    headers: { 
      'Authorization': `Bearer ${localStorage.getItem('token')}`,
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
  await fetch(`/api/articles/${id}/star`, {
    method: 'POST',
    headers: { 
      'Authorization': `Bearer ${localStorage.getItem('token')}`,
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

onMounted(fetchArticles)
watch(() => [props.feedId, props.isRead, props.isStarred], () => {
  articleSearch.value = ''
  fetchArticles()
})

const isSidebarVisible = defineModel<boolean>('isSidebarVisible', { default: true })

const selectArticle = (article: any) => {
  selectedArticle.value = article
  fetchArticleDetail(article.id)
  isSidebarVisible.value = false // 点击文章后收起列表
}

const formatDate = (ts: number) => {
  return new Date(ts * 1000).toLocaleString()
}
</script>

<template>
  <div class="article-view d-flex h-100 overflow-hidden">
    <!-- Article List -->
    <div 
      class="article-list border-e transition-sidebar" 
      :class="{ 'sidebar-hidden': !isSidebarVisible }"
    >
      <!-- 搜索栏 -->
      <div class="article-search-box">
        <v-text-field
          v-model="articleSearch"
          placeholder="搜索文章..."
          variant="outlined"
          density="compact"
          rounded="xl"
          hide-details
          clearable
          bg-color="surface"
          color="primary"
        >
          <template #prepend-inner>
            <v-icon size="18" color="medium-emphasis">mdi-magnify</v-icon>
          </template>
        </v-text-field>
      </div>

      <v-list lines="two" class="pa-0 article-list-scroll custom-scrollbar">
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
              >
                {{ article.isRead ? 'mdi-check-circle' : 'mdi-circle-outline' }}
              </v-icon>
              <span class="text-caption text-medium-emphasis">{{ article.feedTitle }}</span>
              <v-spacer />
              <v-icon v-if="article.isStarred" size="x-small" color="warning">mdi-star</v-icon>
            </div>
            <div class="text-subtitle-2 font-weight-bold line-clamp-2" :class="article.isRead ? 'text-medium-emphasis' : ''">{{ article.title }}</div>
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
          <v-icon size="36" class="mb-3" color="medium-emphasis">mdi-text-search</v-icon>
          <span class="text-body-2">没有找到匹配的文章</span>
          <v-btn
            variant="text"
            color="primary"
            size="small"
            class="mt-2 text-none"
            @click="articleSearch = ''"
          >清除搜索</v-btn>
        </div>

        <div v-if="loading" class="d-flex justify-center pa-4">
          <v-progress-circular indeterminate size="24" width="2" color="primary" />
        </div>
      </v-list>
    </div>

    <!-- Article Content -->
    <div class="article-content flex-grow-1 overflow-y-auto px-6 py-8 custom-scrollbar bg-surface">
      <div v-if="selectedArticle && articleDetail" class="mx-auto" style="max-width: 800px;">
        <div class="d-flex align-start justify-space-between mb-2">
            <h1 class="text-h4 font-weight-bold flex-grow-1 mr-4">{{ articleDetail.title }}</h1>
            <div class="d-flex">
                <v-btn
                    icon
                    variant="text"
                    :color="articleDetail.isStarred ? 'warning' : ''"
                    @click="toggleStar(articleDetail.id, articleDetail.isStarred)"
                >
                    <v-icon>{{ articleDetail.isStarred ? 'mdi-star' : 'mdi-star-outline' }}</v-icon>
                </v-btn>
                <v-btn
                    icon
                    variant="text"
                    :color="articleDetail.isRead ? 'success' : 'primary'"
                    @click="updateReadStatus(articleDetail.id, !articleDetail.isRead)"
                    :title="articleDetail.isRead ? '标记为未读' : '标记为已读'"
                >
                    <v-icon>{{ articleDetail.isRead ? 'mdi-check-circle' : 'mdi-circle-outline' }}</v-icon>
                </v-btn>
                <v-btn
                    v-if="articleDetail.link"
                    variant="text"
                    icon="mdi-open-in-new"
                    :href="articleDetail.link"
                    target="_blank"
                />
            </div>
        </div>
        
        <div class="d-flex align-center mb-12 text-medium-emphasis py-6 border-y" style="border-color: rgba(var(--v-border-color), 0.08) !important;">
          <span v-if="articleDetail.author" class="d-flex align-center mr-12">
            <v-icon size="small" class="mr-3">mdi-account-outline</v-icon>
            {{ articleDetail.author }}
          </span>
          <span class="d-flex align-center">
            <v-icon size="small" class="mr-3">mdi-clock-outline</v-icon>
            {{ articleDetail.publishedAt ? formatDate(articleDetail.publishedAt) : '' }}
          </span>
        </div>

        <ArticleContent :content="stitchedContent" />
      </div>
      <div v-else class="h-100 d-flex flex-column align-center justify-center text-medium-emphasis opacity-60">
        <v-icon size="64" class="mb-4">mdi-newspaper-variant-outline</v-icon>
        <p>选择一篇文章预览内容</p>
      </div>
    </div>
  </div>
</template>

<style>
.article-view {
  height: calc(100vh - 180px); /* Adjust based on App Bar */
  margin: -32px; /* Pull out to edges of container */
}

.transition-sidebar {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  width: 350px;
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
  padding: 10px 12px 8px;
  background: rgb(var(--v-theme-surface));
  border-bottom: 1px solid rgba(var(--v-border-color), 0.06);
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

</style>

