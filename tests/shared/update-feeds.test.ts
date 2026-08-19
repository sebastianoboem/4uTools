import { afterEach, describe, expect, it } from 'vitest'
import {
  GITHUB_UPDATE_FEED,
  genericFeed,
  localUpdateFeedUrl
} from '../../src/shared/update-feeds'

describe('update-feeds', () => {
  afterEach(() => {
    delete process.env.GFM_UPDATE_FEED_URL
  })

  it('uses GitHub as the production feed identity', () => {
    expect(GITHUB_UPDATE_FEED).toEqual({
      provider: 'github',
      owner: 'sebastianoboem',
      repo: 'GoogleFotoManager'
    })
  })

  it('normalizes generic feed URLs with a trailing slash', () => {
    expect(genericFeed('http://127.0.0.1:8765')).toEqual({
      provider: 'generic',
      url: 'http://127.0.0.1:8765/'
    })
  })

  it('reads optional local staging feed from env', () => {
    process.env.GFM_UPDATE_FEED_URL = ' http://127.0.0.1:8765 '
    expect(localUpdateFeedUrl()).toBe('http://127.0.0.1:8765')
  })
})
