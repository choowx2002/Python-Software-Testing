import pluginVue from 'eslint-plugin-vue'
import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript'
import skipFormatting from '@vue/eslint-config-prettier/skip-formatting'

export default defineConfigWithVueTs(
  pluginVue.configs['flat/essential'],
  vueTsConfigs.recommended,
  skipFormatting,
  {
    // 构建产物与 pytest 临时缓存目录不参与 lint
    ignores: ['dist/**', 'src-tauri/target/**', 'pytest-cache-files-*/**'],
  },
  {
    files: ['**/*.vue', '**/*.ts', '**/*.tsx', '**/*.js'],
    rules: {
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  }
)
