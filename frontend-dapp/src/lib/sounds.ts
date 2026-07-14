import { readSoundsEnabled } from '@/utils/soundPreferences'

const SOUNDS = {
  buttonPress: '/sounds/button-press.wav',
  hover: '/sounds/hover.wav',
  success: '/sounds/success.wav',
  error: '/sounds/error.wav',
} as const

let buttonPressAudio: HTMLAudioElement | null = null
let hoverAudio: HTMLAudioElement | null = null
let successAudio: HTMLAudioElement | null = null
let errorAudio: HTMLAudioElement | null = null

function getAudio(key: keyof typeof SOUNDS): HTMLAudioElement | null {
  switch (key) {
    case 'buttonPress':
      if (!buttonPressAudio) buttonPressAudio = new Audio(SOUNDS.buttonPress)
      return buttonPressAudio
    case 'hover':
      if (!hoverAudio) hoverAudio = new Audio(SOUNDS.hover)
      return hoverAudio
    case 'success':
      if (!successAudio) successAudio = new Audio(SOUNDS.success)
      return successAudio
    case 'error':
      if (!errorAudio) errorAudio = new Audio(SOUNDS.error)
      return errorAudio
  }
}

/**
 * Single mute gate for all SFX kinds (GitLab #487).
 * Call sites must keep using `sounds.play*()` — do not fork mute checks per import.
 */
function play(key: keyof typeof SOUNDS): void {
  if (!readSoundsEnabled()) return
  const audio = getAudio(key)
  if (!audio) return
  try {
    audio.currentTime = 0
    audio.volume = key === 'buttonPress' ? 0.2 : 0.4
    audio.play().catch(() => {})
  } catch {
    // ignore
  }
}

export const sounds = {
  playButtonPress: () => play('buttonPress'),
  playHover: () => play('hover'),
  playSuccess: () => play('success'),
  playError: () => play('error'),
}

/** Test-only: drop lazy Audio singletons so stubs rebind cleanly. */
export function resetSoundsAudioForTests(): void {
  buttonPressAudio = null
  hoverAudio = null
  successAudio = null
  errorAudio = null
}
