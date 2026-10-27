import { watch } from 'vue'
import { createI18n } from 'vue-i18n'
import zh from './locales/zh.json'
import en from './locales/en.json'

const initialLocale = localStorage.getItem('locale') || 'zh'

const syncDocumentLang = (locale: string) => {
  document.documentElement.lang = locale === 'zh' ? 'zh-CN' : 'en'
}

const i18n = createI18n({
  legacy: false, // 使用 Composition API
  locale: initialLocale, // 从本地存储读取
  fallbackLocale: 'en',
  messages: {
    zh,
    en
  }
})

syncDocumentLang(initialLocale)
watch(i18n.global.locale, (value) => syncDocumentLang(String(value)))

export default i18n
