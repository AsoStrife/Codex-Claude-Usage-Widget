import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import SettingsPanel from './SettingsPanel.vue'
import { defaultSettings } from '../types/usage'

describe('monitor selection', () => {
  it('allows every provider to be hidden even when its CLI is not installed', async () => {
    const panel = mount(SettingsPanel, { props: { settings: defaultSettings(), diagnostics: null, busy: false, now: 0, error: null } })
    for (const [label, provider] of [['Codex', 'codex'], ['Claude', 'claude'], ['Antigravity', 'antigravity']]) {
      const toggle = panel.findAll('[role="switch"]').find((button) => button.text() === label)!
      expect(toggle.attributes('disabled')).toBeUndefined()
      await toggle.trigger('click')
      expect(panel.emitted('set-provider-monitoring')).toContainEqual([provider, false])
    }
    expect(panel.emitted('set-claude-integration')).toBeUndefined()
    expect(panel.emitted('set-antigravity-integration')).toBeUndefined()
  })
})
