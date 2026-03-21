import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import VueInspector from 'vite-plugin-vue-inspector' // [FactCheck] 导入插件

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    vuetify({ autoImport: true }),
    // [FactCheck] 建议配置：
    VueInspector({
      enabled: true,
      toggleComboKey: 'control-shift', // 推荐快捷键，避免与浏览器默认快捷键冲突
      appendTo: 'main.ts', // 确保注入到入口文件
    }),
  ],
  server: {
    port: 8001,
    host: '0.0.0.0',
    headers: {
      'Cache-Control': 'no-store',
    },
    fs: {
      allow: ['..'],
    },
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8000',
        changeOrigin: true,
      },
    }
  },
})