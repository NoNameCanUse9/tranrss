<script setup lang="ts">
/**
 * ArticleContent
 * 
 * 文章正文渲染组件。负责将后端拼接好的 HTML 内容（含双语标记）渲染为可读界面。
 * 
 * 双语翻译的 HTML 结构（由后端 get_article 生成）：
 *   原文文字<br><em class="trans-text">译文</em>
 * 
 * 您可以在此组件的 <style> 块中自由修改 .trans-text 的样式，
 * 所有文章的翻译展示效果都会同步变化。
 */
import { computed } from 'vue'
import DOMPurify from 'dompurify'

const props = defineProps<{
  content: string
  customStyle?: string
}>()

// RSS/文章内容来自外部订阅源，必须在渲染前净化，避免 <img onerror> /
// javascript: 链接等注入。DOMPurify 默认配置保留常规标签、class 与图片。
const sanitizedContent = computed(() => DOMPurify.sanitize(props.content))
</script>

<template>
  <div class="article-content-body-wrapper">
    <!-- Inject user custom style for translated text with high specificity -->
    <component :is="'style'" v-if="customStyle">
      /* 增加 ID 选择器前缀以提升优先级，确保覆盖组件默认 style */
      #app .article-content-body em.trans-text {
        {{ customStyle }}
      }
    </component>
    <div class="article-content-body" v-html="sanitizedContent" />
  </div>
</template>

<style scoped>
/* =============================================
   文章正文基础样式
   ============================================= */
.article-content-body {
  font-size: 1.1rem;
  line-height: 1.8;
  color: rgb(var(--v-theme-on-surface));
}

.article-content-body :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: 12px;
  margin: 1.5rem 0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.article-content-body :deep(p) {
  margin-bottom: 1.5rem;
}

.article-content-body :deep(h2),
.article-content-body :deep(h3) {
  margin: 2rem 0 1rem;
  font-weight: bold;
}

.article-content-body :deep(a) {
  color: rgb(var(--v-theme-primary));
  text-decoration: none;
}

.article-content-body :deep(blockquote) {
  border-left: 4px solid rgb(var(--v-theme-primary));
  padding-left: 1.5rem;
  margin: 1.5rem 0;
  font-style: italic;
  opacity: 0.8;
}

/* =============================================
   ✏️ 翻译样式 - 已移至数据库管理，此处留空用于兼容
   ============================================= */
.article-content-body :deep(em.trans-text) {
  /* 默认样式现由数据库 custom_trans_style 提供并通过动态 <style> 注入 */
}
</style>
