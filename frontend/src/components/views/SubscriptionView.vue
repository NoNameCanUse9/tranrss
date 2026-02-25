<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

interface Subscription {
  id: number
  title: string
  url: string
  category: string
  articleCount: number
  lastSync: string | null
  status: 'active' | 'error'
  targetLanguage: string
  autoTranslate: boolean
  needSummary: boolean
  siteUrl?: string | null
  description?: string | null
  iconUrl?: string | null
}

const getHostname = (url: string) => {
  try {
    return new URL(url).hostname
  } catch (e) {
    return url
  }
}

const subscriptions = ref<Subscription[]>([])
const loading = ref(true)
const error = ref('')

const fetchSubscriptions = async () => {
  loading.value = true
  try {
    const response = await fetch('/api/subscriptions', {
      headers: {
        'Authorization': `Bearer ${localStorage.getItem('token')}`
      }
    })
    if (!response.ok) throw new Error('无法加载订阅')
    subscriptions.value = await response.json()
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

onMounted(fetchSubscriptions)

const dialog = ref(false)
const deleteDialog = ref(false)
const selectedSub = ref<Subscription | null>(null)
const search = ref('')
const syncing = ref<Record<number, boolean>>({})
const saving = ref(false)
const fetchingPreview = ref(false)
const snackbar = ref({ show: false, text: '', color: 'success' })

const categories = ['技术', '科技媒体', '研究', '新闻', '财经', '其他']

const form = ref({
  title: '',
  url: '',
  category: '技术',
  targetLanguage: 'zh',
  autoTranslate: false,
  needSummary: false,
  siteUrl: '',
  description: '',
  iconUrl: '',
})

const filtered = computed(() => {
  if (!search.value) return subscriptions.value
  const q = search.value.toLowerCase()
  return subscriptions.value.filter(
    s => s.title.toLowerCase().includes(q) || s.url.toLowerCase().includes(q) || s.category.toLowerCase().includes(q)
  )
})

const statusInfo = (status: string): { color: string; label: string; icon: string } => {
  const map: Record<string, { color: string; label: string; icon: string }> = {
    active: { color: 'success', label: '正常', icon: 'mdi-check-circle-outline' },
    error: { color: 'error', label: '错误', icon: 'mdi-alert-circle-outline' },
  }
  return map[status] || map['active']!
}

const syncNow = async (id: number) => {
  syncing.value[id] = true
  try {
    const response = await fetch(`/api/subscriptions/${id}/sync`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${localStorage.getItem('token')}`
      }
    })
    if (!response.ok) throw new Error('同步失败')
    
    snackbar.value = { show: true, text: '同步成功', color: 'success' }
    await fetchSubscriptions()
  } catch (e: any) {
    snackbar.value = { show: true, text: e.message, color: 'error' }
  } finally {
    syncing.value[id] = false
  }
}

const openAddDialog = () => {
  form.value = { 
    title: '', url: '', category: '技术', targetLanguage: 'zh', autoTranslate: false, needSummary: false,
    siteUrl: '', description: '', iconUrl: ''
  }
  selectedSub.value = null
  dialog.value = true
}

const openEditDialog = (sub: Subscription) => {
  form.value = { 
    title: sub.title, 
    url: sub.url, 
    category: sub.category, 
    targetLanguage: sub.targetLanguage || 'zh',
    autoTranslate: sub.autoTranslate,
    needSummary: sub.needSummary,
    siteUrl: sub.siteUrl || '',
    description: sub.description || '',
    iconUrl: sub.iconUrl || '',
  }
  selectedSub.value = sub
  dialog.value = true
}

const confirmDelete = (sub: Subscription) => {
  selectedSub.value = sub
  deleteDialog.value = true
}

const deleteSub = async () => {
  if (!selectedSub.value) return
  
  try {
    const response = await fetch(`/api/subscriptions/${selectedSub.value.id}`, {
      method: 'DELETE',
      headers: {
        'Authorization': `Bearer ${localStorage.getItem('token')}`
      }
    })
    if (!response.ok) throw new Error('删除失败')
    await fetchSubscriptions()
  } catch (e) {
    console.error(e)
  }
  deleteDialog.value = false
}

const saveSub = async () => {
  saving.value = true
  try {
    const isEdit = !!selectedSub.value
    const url = isEdit ? `/api/subscriptions/${selectedSub.value!.id}` : '/api/subscriptions'
    const method = isEdit ? 'PUT' : 'POST'
    
    // 调整参数名以匹配后端模型
    const payload = {
      feedUrl: form.value.url,
      customTitle: form.value.title,
      needTranslate: form.value.autoTranslate,
      needSummary: form.value.needSummary,
      siteUrl: form.value.siteUrl,
      description: form.value.description,
      iconUrl: form.value.iconUrl,
      targetLanguage: form.value.targetLanguage,
      // folderId: ... // 暂时默认未分类
    }

    const response = await fetch(url, {
      method,
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${localStorage.getItem('token')}`
      },
      body: JSON.stringify(payload),
    })

    if (!response.ok) throw new Error('保存失败')
    
    await fetchSubscriptions()
    dialog.value = false
  } catch (e) {
    console.error(e)
  } finally {
    saving.value = false
  }
}

const formatDate = (dateStr: string | null) => {
  if (!dateStr) return '从未'
  const date = new Date(dateStr)
  return date.toLocaleString('zh-CN', { hour12: false }).replace(',', '')
}

const handleUrlBlur = async () => {
  if (!form.value.url || selectedSub.value) return
  
  fetchingPreview.value = true
  try {
    const response = await fetch('/api/subscriptions/preview', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${localStorage.getItem('token')}`
      },
      body: JSON.stringify({ url: form.value.url })
    })
    
    if (!response.ok) throw new Error('获取订阅信息失败')
    
    const data = await response.json()
    if (data.title && !form.value.title) {
      form.value.title = data.title
    }
    form.value.siteUrl = data.siteUrl || ''
    form.value.description = data.description || ''
    form.value.iconUrl = data.iconUrl || ''
  } catch (e: any) {
    snackbar.value = {
      show: true,
      text: e.message || '获取订阅信息失败，请检查 URL 是否正确',
      color: 'error'
    }
  } finally {
    fetchingPreview.value = false
  }
}
</script>

<template>
  <div class="subscription-view">
    <div class="d-flex align-center justify-space-between mb-8">
      <div>
        <h1 class="text-h3 font-weight-bold">订阅源</h1>
        <p class="text-body-1 text-medium-emphasis mt-2">共 {{ subscriptions.length }} 个活动的源</p>
      </div>
      <v-btn color="primary" rounded="pill" elevation="0" class="text-none font-weight-bold" @click="openAddDialog">
        <v-icon start>mdi-plus</v-icon>
        添加订阅
      </v-btn>
    </div>

    <!-- 搜索 -->
    <v-text-field
      v-model="search"
      placeholder="搜索订阅..."
      variant="outlined"
      density="comfortable"
      rounded="xl"
      class="mb-4"
      color="primary"
      prepend-inner-icon="mdi-magnify"
      hide-details
      clearable
    />

    <!-- 空状态 -->
    <v-card v-if="filtered.length === 0" rounded="xl" variant="tonal" color="surface-variant" class="text-center pa-12">
      <v-icon size="64" color="primary" class="mb-4">mdi-rss-box</v-icon>
      <h3 class="text-h6 mb-2">{{ search ? '未找到匹配订阅' : '暂无订阅源' }}</h3>
      <p class="text-body-2 text-medium-emphasis mb-6">{{ search ? '请尝试其他关键词' : '添加RSS订阅源开始阅读' }}</p>
      <v-btn v-if="!search" color="primary" rounded="pill" elevation="0" class="text-none" @click="openAddDialog">
        添加第一个订阅
      </v-btn>
    </v-card>

    <v-row v-else>
      <v-col v-for="sub in filtered" :key="sub.id" cols="12" md="6" lg="4">
        <v-card rounded="xl" variant="flat" color="surface" class="sub-card h-100">
          <v-card-text class="pa-5">
            <div class="d-flex align-start justify-space-between mb-2">
              <div class="flex-1 min-w-0 mr-2">
                <p class="text-body-1 font-weight-semibold text-truncate">{{ sub.title }}</p>
                <p class="text-caption text-medium-emphasis text-truncate">{{ sub.url }}</p>
              </div>
              <v-chip
                :color="statusInfo(sub.status).color"
                size="x-small"
                variant="tonal"
                class="text-none flex-shrink-0"
              >
                <v-icon start size="12">{{ statusInfo(sub.status).icon }}</v-icon>
                {{ statusInfo(sub.status).label }}
              </v-chip>
            </div>

            <div class="d-flex align-center gap-2 mb-3">
              <v-chip size="x-small" variant="outlined" color="primary" class="text-none">
                <v-icon start size="12">mdi-tag-outline</v-icon>
                {{ sub.category }}
              </v-chip>
              <v-chip v-if="sub.autoTranslate" size="x-small" variant="outlined" color="secondary" class="text-none">
                <v-icon start size="12">mdi-translate</v-icon>
                自动翻译
              </v-chip>
              <v-chip v-if="sub.needSummary" size="x-small" variant="outlined" color="info" class="text-none ml-1">
                <v-icon start size="12">mdi-text-box-search-outline</v-icon>
                简报
              </v-chip>
            </div>

            <v-divider class="mb-3" />
            
            <div v-if="sub.description" class="text-caption text-medium-emphasis line-clamp-2 mb-3">
              {{ sub.description }}
            </div>

            <div class="d-flex align-center gap-6 text-caption text-medium-emphasis mb-4">
              <span class="d-flex align-center"><v-icon size="14" class="mr-1">mdi-newspaper-outline</v-icon>{{ sub.articleCount }} 篇</span>
              <span class="d-flex align-center text-truncate"><v-icon size="14" class="mr-1">mdi-clock-outline</v-icon>{{ formatDate(sub.lastSync) }}</span>
            </div>

            <div class="d-flex align-center gap-2">
              <v-btn
                variant="tonal"
                color="primary"
                size="small"
                rounded="pill"
                class="text-none flex-1"
                :loading="syncing[sub.id]"
                @click="syncNow(sub.id)"
              >
                <v-icon start size="16">mdi-refresh</v-icon>
                同步
              </v-btn>
              <v-btn 
                variant="tonal" 
                color="secondary" 
                size="small" 
                rounded="circle" 
                class="text-none" 
                min-width="36"
                @click="openEditDialog(sub)"
              >
                <v-icon size="16">mdi-pencil-outline</v-icon>
              </v-btn>
              <v-btn 
                variant="tonal" 
                color="error" 
                size="small" 
                rounded="circle" 
                class="text-none" 
                min-width="36"
                @click="confirmDelete(sub)"
              >
                <v-icon size="16">mdi-trash-can-outline</v-icon>
              </v-btn>
            </div>
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>

    <!-- 添加/编辑对话框 -->
    <v-dialog v-model="dialog" width="60%" :scrim="true" persistent scrollable>
      <v-card rounded="xl" class="subscription-dialog shadow-premium">
        <div class="dialog-header pa-6 d-flex align-center justify-space-between">
          <div>
            <h2 class="text-h5 font-weight-bold gradient-text">
              {{ selectedSub ? '配置订阅源' : '添加新订阅' }}
            </h2>
            <p class="text-caption text-medium-emphasis mt-1">
              {{ selectedSub ? '更新您的阅读偏好与订阅元数据' : '输入 RSS 链接，开启您的智能阅读之旅' }}
            </p>
          </div>
          <v-btn icon="mdi-close" variant="text" color="error" rounded="pill" @click="dialog = false"></v-btn>
        </div>

        <v-divider />

        <v-card-text class="pa-8 custom-scrollbar" style="max-height: 70vh;">
          <div class="d-flex flex-column gap-8">
            <!-- 第 1 步：订阅源基本信息 -->
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="primary" class="mr-2">mdi-rss</v-icon>
                订阅源元数据
              </h3>
              
              <div class="d-flex flex-column gap-6">
                <!-- 实时预览卡片 -->
                <div class="preview-card pa-6 rounded-xl text-center d-flex flex-column align-center gap-2 mb-2">
                  <v-avatar size="80" rounded="xl" class="shadow-sm mb-2" color="white">
                    <v-img :src="form.iconUrl || ''" cover>
                      <template v-slot:placeholder>
                        <v-icon size="40" color="grey-lighten-2">mdi-rss-box</v-icon>
                      </template>
                    </v-img>
                  </v-avatar>
                  <div v-if="form.title || form.url" class="text-h6 font-weight-bold text-truncate max-w-100">
                    {{ form.title || '新订阅源' }}
                  </div>
                  <div v-if="form.siteUrl" class="text-body-2 text-primary text-decoration-none d-flex align-center">
                    <v-icon size="14" class="mr-1">mdi-link-variant</v-icon>
                    {{ getHostname(form.siteUrl) }}
                  </div>
                </div>

                <div class="d-flex flex-column gap-4">
                  <v-text-field 
                    v-model="form.url" 
                    label="RSS 订阅地址" 
                    placeholder="https://example.com/feed.xml"
                    variant="outlined" 
                    density="comfortable" 
                    rounded="lg" 
                    color="primary" 
                    prepend-inner-icon="mdi-rss" 
                    messages="支持标准 RSS, Atom 和 JSON Feed"
                    @blur="handleUrlBlur"
                    :loading="fetchingPreview"
                  />
                  
                  <v-row>
                    <v-col cols="12" sm="6">
                      <v-text-field 
                        v-model="form.siteUrl" 
                        label="源站点链接" 
                        variant="outlined" 
                        density="comfortable" 
                        rounded="lg" 
                        color="primary" 
                        prepend-inner-icon="mdi-earth" 
                        hide-details 
                      />
                    </v-col>
                    <v-col cols="12" sm="6">
                      <v-text-field 
                        v-model="form.iconUrl" 
                        label="图标 URL" 
                        variant="outlined" 
                        density="comfortable" 
                        rounded="lg" 
                        color="primary" 
                        prepend-inner-icon="mdi-image-outline" 
                        hide-details 
                      />
                    </v-col>
                  </v-row>

                  <v-textarea 
                    v-model="form.description" 
                    label="源描述信息" 
                    variant="outlined" 
                    density="comfortable" 
                    rounded="lg" 
                    color="primary" 
                    prepend-inner-icon="mdi-text-subject" 
                    hide-details 
                    rows="2"
                    placeholder="该订阅源的简要介绍..."
                  />
                </div>
              </div>

              <v-alert
                v-if="!form.url"
                type="info"
                variant="tonal"
                density="compact"
                icon="mdi-information-outline"
                class="text-caption mt-6"
              >
                输入 RSS 地址后，我们将自动为您提取这些信息。
              </v-alert>
            </section>

            <v-divider />

            <!-- 第 2 步：个人订阅配置 -->
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="secondary" class="mr-2">mdi-cog-outline</v-icon>
                阅读偏好设置
              </h3>
              <div class="d-flex flex-column gap-4">
                <v-row>
                  <v-col cols="12" sm="6">
                    <v-text-field 
                      v-model="form.title" 
                      label="自定义显示名称" 
                      placeholder="如果不填写将使用源标题"
                      variant="outlined" 
                      density="comfortable" 
                      rounded="lg" 
                      color="primary" 
                      prepend-inner-icon="mdi-pencil-outline" 
                      hide-details
                    />
                  </v-col>
                  <v-col cols="12" sm="6">
                    <v-combobox 
                      v-model="form.category" 
                      :items="categories" 
                      label="所属分类" 
                      placeholder="选择或输入新分类"
                      variant="outlined" 
                      density="comfortable" 
                      rounded="lg" 
                      color="primary" 
                      prepend-inner-icon="mdi-folder-outline" 
                      hide-details 
                      hide-no-data
                    />
                  </v-col>
                </v-row>
              </div>
            </section>

            <v-divider />

            <!-- 第 步：AI 智能增强 -->
            <section>
              <h3 class="text-subtitle-1 font-weight-bold mb-4 d-flex align-center">
                <v-icon color="info" class="mr-2">mdi-robot-outline</v-icon>
                AI 智能增强
              </h3>
              <v-card variant="tonal" border color="primary" rounded="lg" class="pa-4 bg-primary-lighten-5">
                <div class="d-flex align-center justify-space-between mb-4">
                  <div class="d-flex align-center">
                    <v-avatar color="primary" size="32" class="mr-3">
                      <v-icon size="18" color="white">mdi-translate</v-icon>
                    </v-avatar>
                    <div>
                      <p class="text-body-2 font-weight-bold">全自动翻译</p>
                      <p class="text-caption text-medium-emphasis">使用 AI 将文章自动翻译为中文（双语对照）</p>
                    </div>
                  </div>
                  <v-switch v-model="form.autoTranslate" color="primary" hide-details density="compact" />
                </div>
                
                <v-divider class="my-3 opacity-20" />

                <div class="d-flex align-center justify-space-between">
                  <div class="d-flex align-center">
                    <v-avatar color="secondary" size="32" class="mr-3">
                      <v-icon size="18" color="white">mdi-auto-fix</v-icon>
                    </v-avatar>
                    <div>
                      <p class="text-body-2 font-weight-bold">智能简报摘要</p>
                      <p class="text-caption text-medium-emphasis">生成 200 字以内的核心内容摘要</p>
                    </div>
                  </div>
                  <v-switch v-model="form.needSummary" color="secondary" hide-details density="compact" />
                </div>

                <!-- 目标语言选择：仅在开启 AI 功能时显示 -->
                <v-expand-transition>
                  <div v-if="form.autoTranslate || form.needSummary">
                    <v-divider class="my-4" />
                    <div class="d-flex align-center gap-4">
                      <v-icon color="primary" size="20">mdi-translate</v-icon>
                      <v-select 
                        v-model="form.targetLanguage" 
                        :items="[
                          { title: '简体中文 (Chinese)', value: 'zh' },
                          { title: '英语 (English)', value: 'en' },
                          { title: '日语 (Japanese)', value: 'ja' },
                          { title: '法语 (French)', value: 'fr' }
                        ]" 
                        label="翻译与简报的目标语言" 
                        variant="outlined" 
                        density="compact" 
                        rounded="lg" 
                        color="primary" 
                        hide-details 
                        class="flex-1"
                      />
                    </div>
                  </div>
                </v-expand-transition>
              </v-card>
            </section>
          </div>
        </v-card-text>

        <v-divider />

        <v-card-actions class="pa-6">
          <v-spacer />
          
          <v-btn 
            color="primary" 
            class="text-none font-weight-bold px-10 btn-premium" 
            rounded="pill" 
            elevation="4" 
            :loading="saving" 
            @click="saveSub"
          >
            {{ selectedSub ? '保存修改' : '立即订阅' }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 删除确认 -->
    <v-dialog v-model="deleteDialog" max-width="360">
      <v-card rounded="xl">
        <v-card-title class="pa-6 pb-2 text-body-1 font-weight-bold">确认删除</v-card-title>
        <v-card-text class="pa-6 pt-2 text-body-2 text-medium-emphasis">
          确定要删除订阅「{{ selectedSub?.title }}」吗？相关文章数据将一并删除。
        </v-card-text>
        <v-card-actions class="pa-6 pt-0">
          <v-spacer />
          <v-btn variant="text" class="text-none" rounded="pill" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" class="text-none font-weight-bold px-6" rounded="pill" elevation="0" @click="deleteSub">删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
    <!-- 提示消息 -->
    <v-snackbar v-model="snackbar.show" :color="snackbar.color" rounded="pill" elevation="12">
      {{ snackbar.text }}
      <template v-slot:actions>
        <v-btn variant="text" @click="snackbar.show = false">关闭</v-btn>
      </template>
    </v-snackbar>
  </div>
</template>

<style scoped>
.sub-card {
  transition: box-shadow 0.2s ease;
}
.sub-card:hover {
  box-shadow: 0 4px 20px rgba(var(--v-theme-primary), 0.12) !important;
}
.gap-2 { gap: 8px; }
.gap-4 { gap: 16px; }
.gap-6 { gap: 24px; }
.min-w-0 { min-width: 0; }
.line-clamp-2 {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
}

/* Premium Dialog Styles */
.subscription-dialog {
  display: flex;
  flex-direction: column;
  max-height: 90vh !important;
}
.custom-scrollbar {
  overflow-y: auto;
}
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 10px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(var(--v-theme-primary), 0.3);
}
.gradient-text {
  background: linear-gradient(135deg, var(--v-theme-primary) 0%, #2c3e50 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
.shadow-premium {
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25) !important;
}
.border-e {
  border-right: 1px solid rgba(0, 0, 0, 0.05);
}
.preview-card {
  background: white;
  border: 1px solid rgba(0, 0, 0, 0.05);
  transition: transform 0.3s ease;
}
.preview-card:hover {
  transform: translateY(-2px);
}
.btn-premium {
  letter-spacing: 0.5px;
  text-transform: none;
  background: linear-gradient(135deg, var(--v-theme-primary) 0%, var(--v-theme-secondary) 100%) !important;
}
</style>
