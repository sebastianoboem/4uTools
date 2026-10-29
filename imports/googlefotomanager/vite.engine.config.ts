import { resolve } from 'node:path'
import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    emptyOutDir: true,
    lib: {
      entry: resolve(__dirname, 'src/engine/inject-entry.ts'),
      name: 'GoogleFotoInject',
      formats: ['iife'],
      fileName: 'inject'
    },
    outDir: resolve(__dirname, 'out/engine'),
    rollupOptions: {
      output: {
        inlineDynamicImports: true
      }
    }
  }
})
