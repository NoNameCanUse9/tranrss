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
defineProps<{
  content: string
}>()
</script>

<template>
  <div class="article-content-body" v-html="content" />
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
   ✏️ 翻译样式 - 在这里自定义翻译的展示方式
   ============================================= */
.article-content-body :deep(em.trans-text) {
  display: block;
  font-style: italic;
  color: rgb(var(--v-theme-on-surface));
  opacity: 0.6;
  font-size: 0.95em;
  margin-top: 0.3rem;
  padding-left: 0.75rem;
  border-left: 2px solid rgba(var(--v-theme-primary), 0.4);
}
</style>
