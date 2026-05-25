import { COPY_BUTTON_FAILURE_MESSAGE } from '@/utils/copyButtonCopy'

export type ClipboardWriter = Pick<Clipboard, 'writeText'>

export type CopyToClipboardResult = { ok: true } | { ok: false; message: string }

/**
 * Writes `text` via the Clipboard API. Injectable `clipboard` supports Vitest mocks.
 */
export async function copyToClipboard(
  text: string,
  clipboard: ClipboardWriter = navigator.clipboard
): Promise<CopyToClipboardResult> {
  const trimmed = text.trim()
  if (!trimmed) {
    return { ok: false, message: COPY_BUTTON_FAILURE_MESSAGE }
  }

  try {
    await clipboard.writeText(trimmed)
    return { ok: true }
  } catch {
    return { ok: false, message: COPY_BUTTON_FAILURE_MESSAGE }
  }
}
