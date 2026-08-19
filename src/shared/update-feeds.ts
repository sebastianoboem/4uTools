declare const __GFM_BAKED_UPDATE_FEED_URL__: string | undefined

/** Optional generic feed for local staging (http://127.0.0.1:…/). Never set in production builds. */
export function localUpdateFeedUrl(): string | undefined {
  const baked =
    typeof __GFM_BAKED_UPDATE_FEED_URL__ === 'string'
      ? __GFM_BAKED_UPDATE_FEED_URL__.trim()
      : ''
  const fromEnv = process.env.GFM_UPDATE_FEED_URL?.trim() || ''
  return fromEnv || baked || undefined
}

export const GITHUB_UPDATE_FEED = {
  provider: 'github' as const,
  owner: 'sebastianoboem',
  repo: 'GoogleFotoManager'
}

export function genericFeed(url: string) {
  const normalized = url.endsWith('/') ? url : `${url}/`
  return {
    provider: 'generic' as const,
    url: normalized
  }
}
