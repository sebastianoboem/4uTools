import { resolve } from 'node:path'
import { defineConfig, externalizeDepsPlugin } from 'electron-vite'

const bakedLocalFeed = process.env.GFM_UPDATE_FEED_URL?.trim() || ''

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
    define: {
      __GFM_BAKED_UPDATE_FEED_URL__: JSON.stringify(bakedLocalFeed)
    },
    build: {
      rollupOptions: {
        input: {
          index: resolve(__dirname, 'src/main/index.ts')
        }
      }
    }
  },
  preload: {
    plugins: [externalizeDepsPlugin()],
    build: {
      rollupOptions: {
        input: {
          panel: resolve(__dirname, 'src/preload/panel.ts'),
          photos: resolve(__dirname, 'src/preload/photos.ts'),
          log: resolve(__dirname, 'src/preload/log.ts'),
          'tutorial-overlay': resolve(__dirname, 'src/preload/tutorial-overlay.ts')
        }
      }
    }
  },
  renderer: {
    build: {
      rollupOptions: {
        input: {
          panel: resolve(__dirname, 'src/renderer/panel/index.html'),
          log: resolve(__dirname, 'src/renderer/log/index.html'),
          'tutorial-overlay': resolve(__dirname, 'src/renderer/tutorial-overlay/index.html'),
          'chrome-placeholder': resolve(
            __dirname,
            'src/renderer/chrome-placeholder/index.html'
          )
        }
      }
    }
  }
})
