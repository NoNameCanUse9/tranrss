import { createI18n } from 'vue-i18n'
import zh from './locales/zh.json'
import en from './locales/en.json'

const i18n = createI18n({
  legacy: false, // 使用 Composition API
  locale: localStorage.getItem('locale') || 'zh', // 从本地存储读取
  fallbackLocale: 'en',
  messages: {
    zh,
    en
  }
})

export default i18n
