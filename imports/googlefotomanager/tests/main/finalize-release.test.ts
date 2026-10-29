import { describe, expect, it } from 'vitest'
import { githubAssetName } from '../../scripts/rename-artifacts.cjs'

describe('githubAssetName', () => {
  it('matches GitHub space-to-dot asset naming', () => {
    expect(githubAssetName('Google Foto Manager-1.2.1.exe')).toBe(
      'Google.Foto.Manager-1.2.1.exe'
    )
  })
})
