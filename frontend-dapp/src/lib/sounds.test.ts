import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { resetSoundsAudioForTests, sounds } from './sounds'
import { resetSoundsEnabledCacheForTests, writeSoundsEnabled } from '@/utils/soundPreferences'

describe('sounds mute gate', () => {
  let playSpy: ReturnType<typeof vi.fn>

  beforeEach(() => {
    window.localStorage.clear()
    resetSoundsEnabledCacheForTests()
    resetSoundsAudioForTests()
    playSpy = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal(
      'Audio',
      class MockAudio {
        currentTime = 0
        volume = 1
        play = playSpy
      }
    )
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    window.localStorage.clear()
    resetSoundsEnabledCacheForTests()
    resetSoundsAudioForTests()
  })

  it('plays when sounds are enabled by default', () => {
    sounds.playButtonPress()
    expect(playSpy).toHaveBeenCalledTimes(1)
  })

  it('does not call audio.play for any kind while muted', () => {
    writeSoundsEnabled(false)
    sounds.playButtonPress()
    sounds.playHover()
    sounds.playSuccess()
    sounds.playError()
    expect(playSpy).not.toHaveBeenCalled()
  })

  it('resumes playback after re-enabling without a reload', () => {
    writeSoundsEnabled(false)
    sounds.playButtonPress()
    expect(playSpy).not.toHaveBeenCalled()

    writeSoundsEnabled(true)
    sounds.playSuccess()
    expect(playSpy).toHaveBeenCalledTimes(1)
  })
})
