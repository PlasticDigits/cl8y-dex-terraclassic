import { describe, expect, it } from 'vitest'
import {
  findPortalListboxTypeaheadIndex,
  movePortalListboxActiveIndex,
  portalListboxOptionId,
  wrapPortalListboxIndex,
} from '../portalListboxKeyboard'

describe('portalListboxKeyboard', () => {
  describe('wrapPortalListboxIndex', () => {
    it('wraps negative and overflow indices', () => {
      expect(wrapPortalListboxIndex(-1, 5)).toBe(4)
      expect(wrapPortalListboxIndex(5, 5)).toBe(0)
    })
  })

  describe('movePortalListboxActiveIndex', () => {
    it('moves down and wraps at end', () => {
      expect(movePortalListboxActiveIndex(4, 1, 5)).toBe(0)
      expect(movePortalListboxActiveIndex(0, -1, 5)).toBe(4)
    })
  })

  describe('findPortalListboxTypeaheadIndex', () => {
    const labels = ['LUNC', 'USTC', 'CL8Y', 'Alpha']

    it('finds first prefix match after start index with wrap', () => {
      expect(findPortalListboxTypeaheadIndex(labels, 'u', 0)).toBe(1)
      expect(findPortalListboxTypeaheadIndex(labels, 'c', 2)).toBe(2)
    })

    it('matches accumulated query prefix case-insensitively', () => {
      expect(findPortalListboxTypeaheadIndex(labels, 'al', 0)).toBe(3)
      expect(findPortalListboxTypeaheadIndex(labels, 'lu', -1)).toBe(0)
    })

    it('returns null when nothing matches', () => {
      expect(findPortalListboxTypeaheadIndex(labels, 'zzz', 0)).toBeNull()
      expect(findPortalListboxTypeaheadIndex([], 'a', 0)).toBeNull()
    })
  })

  describe('portalListboxOptionId', () => {
    it('builds stable option ids for aria-activedescendant', () => {
      expect(portalListboxOptionId(':r1:', 2)).toBe(':r1:-option-2')
    })
  })
})
