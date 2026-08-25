import { describe, expect, it } from 'vitest'
import { optionMovedBeyondThreshold } from '../optionTapStability'

function rect(top: number, left: number): DOMRect {
  return {
    top,
    left,
    bottom: top + 44,
    right: left + 200,
    width: 200,
    height: 44,
    x: left,
    y: top,
    toJSON: () => ({}),
  } as DOMRect
}

describe('optionMovedBeyondThreshold (GitLab #632)', () => {
  it('ignores a tap after the row slides more than 12px', () => {
    expect(optionMovedBeyondThreshold(rect(400, 16), rect(400, 16))).toBe(false)
    expect(optionMovedBeyondThreshold(rect(400, 16), rect(413, 16))).toBe(true)
    expect(optionMovedBeyondThreshold(rect(400, 16), rect(400, 30))).toBe(true)
  })
})
