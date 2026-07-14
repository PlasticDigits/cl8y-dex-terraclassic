# Agent playbook: UI sound mute toggle

Use when changing UI SFX playback, the shell mute control, or `cl8y-dex-sounds-enabled` persistence ([GitLab **#487**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/487)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § UI sound effects mute](../docs/frontend.md#ui-sound-effects-mute) | Invariants + acceptance notes |
| [`soundPreferences.ts`](../frontend-dapp/src/utils/soundPreferences.ts) | `readSoundsEnabled` / `writeSoundsEnabled`, storage key, session cache |
| [`sounds.ts`](../frontend-dapp/src/lib/sounds.ts) | **Single** mute gate inside `play()` for press / hover / success / error |
| [`SoundEffectsToggle.tsx`](../frontend-dapp/src/components/common/SoundEffectsToggle.tsx) | Shell control (`aria-pressed` = sounds enabled) |
| [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) | Desktop header + mobile More placement next to theme |
| [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) §10.3 | Mute / unmute manual cases |

## Rules of thumb

1. **Default ON** — missing/invalid storage → sounds play (historical behavior + QA default).
2. **Gate only in `sounds.ts` `play()`** — never duplicate mute checks at the ~70 call sites; silent-by-omission buttons stay silent.
3. **One preference for all four WAV kinds** — no per-kind mute or volume mixer in this feature.
4. **Persist** with `cl8y-dex-sounds-enabled` as `'1'`/`'0'`; keep try/catch + in-tab session cache so private-mode write failures still mute for the session.
5. **Placement** — beside theme: sticky header on `≥768px`; mobile **More** sheet on `≤767px` (same as theme — see [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md)).
6. **Icon chrome** — theme + sound use compact flat SVGs in [`shellPrefIcons.tsx`](../frontend-dapp/src/components/common/shellPrefIcons.tsx) (moon / sun / speaker); do not restore text labels in the header. Keep explicit `aria-label` / `title`.
7. **Mute UX** — write preference **before** any `play*`; turning **off** must not play a press sound; turning **on** may play one confirmation press (current product choice).
8. **Do not** add a global click→sound interceptor or remote audio URLs outside `public/sounds/`.
9. **Cross-tab desync** until reload is acceptable MVP; optional `storage` event sync is a follow-up.

## Tests

- Unit: [`soundPreferences.test.ts`](../frontend-dapp/src/utils/soundPreferences.test.ts), [`sounds.test.ts`](../frontend-dapp/src/lib/sounds.test.ts), [`SoundEffectsToggle.test.tsx`](../frontend-dapp/src/components/common/__tests__/SoundEffectsToggle.test.tsx).
- Component tests that `vi.mock('@/lib/sounds')` should keep working unchanged.

## Related

- Theme placement: [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md)
- Shell nav density: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Focus rings on shell buttons: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
