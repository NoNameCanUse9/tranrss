import { createApp } from 'vue'
// Vuetify
import 'vuetify/styles'
import './style.css'
import App from './App.vue'
import i18n from './i18n'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import { aliases, mdi } from 'vuetify/iconsets/mdi'
import '@mdi/font/css/materialdesignicons.css'

import { md3 } from 'vuetify/blueprints'

const vuetify = createVuetify({
    blueprint: md3,
    components,
    directives,
    icons: {
        defaultSet: 'mdi',
        aliases,
        sets: {
            mdi,
        },
    },
    theme: {
        defaultTheme: 'light',
        themes: {
            light: {
                colors: {
                    primary: '#16666e',
                    'surface-tint': '#16666e',
                    'on-primary': '#ffffff',
                    'primary-container': '#a6eef5',
                    'on-primary-container': '#002023',
                    secondary: '#4a6266',
                    'on-secondary': '#ffffff',
                    'secondary-container': '#cde7ec',
                    'on-secondary-container': '#051f22',
                    tertiary: '#525e7d',
                    'on-tertiary': '#ffffff',
                    'tertiary-container': '#d9e2ff',
                    'on-tertiary-container': '#0e1b37',
                    error: '#ba1a1a',
                    'on-error': '#ffffff',
                    'error-container': '#ffdad6',
                    'on-error-container': '#410002',
                    background: '#f1f5f7',
                    'on-background': '#191c1d',
                    surface: '#f1f5f7',
                    'on-surface': '#191c1d',
                    'surface-variant': '#dbe4e6',
                    'on-surface-variant': '#3f484a',
                    outline: '#70797b',
                    'outline-variant': '#bfc8ca',
                    shadow: '#000000',
                    scrim: '#000000',
                    'inverse-surface': '#2e3132',
                    'inverse-on-surface': '#eff1f1',
                    'inverse-primary': '#8ad2db',
                    'primary-fixed': '#a6eef5',
                    'on-primary-fixed': '#002023',
                    'primary-fixed-dim': '#8ad2db',
                    'on-primary-fixed-variant': '#004f56',
                    'secondary-fixed': '#cde7ec',
                    'on-secondary-fixed': '#051f22',
                    'secondary-fixed-dim': '#b1cbd0',
                    'on-secondary-fixed-variant': '#324a4f',
                    'tertiary-fixed': '#d9e2ff',
                    'on-tertiary-fixed': '#0e1b37',
                    'tertiary-fixed-dim': '#bac6ea',
                    'on-tertiary-fixed-variant': '#3a4664',
                    'surface-dim': '#d8dadb',
                    'surface-bright': '#f8f9fa',
                    'surface-container-lowest': '#ffffff',
                    'surface-container-low': '#f1f4f5',
                    'surface-container': '#ebf1f3',
                    'surface-container-high': '#dce6e9',
                    'surface-container-highest': '#d1d8db',
                }
            },
            dark: {
                colors: {
                    primary: '#8ad2db',
                    'surface-tint': '#8ad2db',
                    'on-primary': '#00363b',
                    'primary-container': '#004f56',
                    'on-primary-container': '#a6eef5',
                    secondary: '#b1cbd0',
                    'on-secondary': '#1c3438',
                    'secondary-container': '#324a4f',
                    'on-secondary-container': '#cde7ec',
                    tertiary: '#bac6ea',
                    'on-tertiary': '#24304d',
                    'tertiary-container': '#3a4664',
                    'on-tertiary-container': '#d9e2ff',
                    error: '#ffb4ab',
                    'on-error': '#690005',
                    'error-container': '#93000a',
                    'on-error-container': '#ffdad6',
                    background: '#191c1d',
                    'on-background': '#e1e3e3',
                    surface: '#191c1d',
                    'on-surface': '#e1e3e3',
                    'surface-variant': '#3f484a',
                    'on-surface-variant': '#bfc8ca',
                    outline: '#899294',
                    'outline-variant': '#3f484a',
                    shadow: '#000000',
                    scrim: '#000000',
                    'inverse-surface': '#e1e3e3',
                    'inverse-on-surface': '#2e3132',
                    'inverse-primary': '#16666e',
                    'primary-fixed': '#a6eef5',
                    'on-primary-fixed': '#002023',
                    'primary-fixed-dim': '#8ad2db',
                    'on-primary-fixed-variant': '#004f56',
                    'secondary-fixed': '#cde7ec',
                    'on-secondary-fixed': '#051f22',
                    'secondary-fixed-dim': '#b1cbd0',
                    'on-secondary-fixed-variant': '#324a4f',
                    'tertiary-fixed': '#d9e2ff',
                    'on-tertiary-fixed': '#0e1b37',
                    'tertiary-fixed-dim': '#bac6ea',
                    'on-tertiary-fixed-variant': '#3a4664',
                    'surface-dim': '#111415',
                    'surface-bright': '#363a3b',
                    'surface-container-lowest': '#0b0f10',
                    'surface-container-low': '#191c1d',
                    'surface-container': '#1d2021',
                    'surface-container-high': '#272a2b',
                    'surface-container-highest': '#323536',
                }
            }
        }
    }
})

createApp(App).use(vuetify).use(i18n).mount('#app')