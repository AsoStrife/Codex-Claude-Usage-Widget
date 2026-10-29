import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { nextTick, reactive } from 'vue'
import App from './App.vue'
import { invoke, listen } from './lib/tauri'
import { defaultSettings, emptyQuota } from './types/usage'

const savedSettings = reactive(defaultSettings())
const settingsActions = { initialize: vi.fn(), loadDiagnostics: vi.fn(), setProviderMonitoring: vi.fn() }

vi.mock('./lib/tauri', () => ({
  isTauri: true,
  invoke: vi.fn(),
  listen: vi.fn(async () => vi.fn()),
}))

vi.mock('./stores/usage', () => ({
  useUsageStore: () => ({
    codex: emptyQuota('codex'), claude: emptyQuota('claude'), antigravity: emptyQuota('antigravity'), now: 0, refreshing: false, lastError: null,
    initialize: vi.fn(), refreshAll: vi.fn(), dispose: vi.fn(),
  }),
}))

vi.mock('./stores/settings', () => ({
  useSettingsStore: () => ({ settings: savedSettings, busy: false, diagnostics: null, lastError: null, ...settingsActions }),
}))

let wrapper: VueWrapper | undefined

beforeEach(() => {
  vi.clearAllMocks()
  Object.assign(savedSettings, defaultSettings())
  vi.stubGlobal('ResizeObserver', class {
    observe() {}
    disconnect() {}
  })
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    if (command === 'get_popup_pinned') return false
    if (command === 'set_popup_pinned') return args?.pinned
    return undefined
  })
})

afterEach(() => {
  wrapper?.unmount()
  vi.unstubAllGlobals()
})

function render() {
  wrapper = mount(App, {
    attachTo: document.body,
    global: { stubs: { ProviderCard: true, SettingsPanel: true } },
  })
  return wrapper
}

describe('widget pin', () => {
  it('pins and unpins through the native window command without starting a drag', async () => {
    const app = render()
    await flushPromises()
    const pin = app.get('button[aria-pressed]')
    expect(pin.attributes('aria-pressed')).toBe('false')
    await pin.trigger('mousedown', { button: 0 })
    expect(invoke).not.toHaveBeenCalledWith('begin_popup_drag')
    await pin.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('set_popup_pinned', { pinned: true })
    expect(pin.attributes('aria-pressed')).toBe('true')
    expect(pin.attributes('title')).toBe('Unpin widget')
    await pin.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('set_popup_pinned', { pinned: false })
    expect(pin.attributes('aria-pressed')).toBe('false')
  })

  it('reads the native pin state when the view is mounted again', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(true)
    const app = render()
    expect(app.get('button[aria-pressed]').attributes('disabled')).toBeDefined()
    await flushPromises()
    expect(app.get('button[aria-pressed]').attributes('aria-pressed')).toBe('true')
  })

  it('disables the toggle until the native command finishes', async () => {
    const app = render()
    await flushPromises()
    let complete!: (value: boolean) => void
    vi.mocked(invoke).mockImplementationOnce(() => new Promise((resolve) => { complete = resolve }))
    const pin = app.get('button[aria-pressed]')
    await pin.trigger('click')
    expect(pin.attributes('disabled')).toBeDefined()
    expect(pin.attributes('aria-pressed')).toBe('false')
    complete(true)
    await flushPromises()
    expect(pin.attributes('disabled')).toBeUndefined()
    expect(pin.attributes('aria-pressed')).toBe('true')
  })

  it('keeps the previous state and reports a failed native toggle', async () => {
    const app = render()
    await flushPromises()
    vi.mocked(invoke).mockRejectedValueOnce(new Error('window unavailable'))
    const pin = app.get('button[aria-pressed]')
    await pin.trigger('click')
    await flushPromises()
    expect(pin.attributes('aria-pressed')).toBe('false')
    expect(pin.attributes('disabled')).toBeUndefined()
    expect(app.text()).toContain('Unable to change widget pin: Error: window unavailable')
  })
})

describe('provider tabs and monitoring', () => {
  const visibleProviders = (app: VueWrapper) => app.findAllComponents({ name: 'ProviderCard' }).map((card) => card.props('quota').provider)

  it('starts with Codex and Claude and shows Antigravity only on its own tab', async () => {
    const app = render()
    expect(visibleProviders(app)).toEqual(['codex', 'claude'])
    const tabs = app.findAll('[role="tab"]')
    expect(tabs.map((tab) => tab.text())).toEqual(['Codex & Claude', 'Antigravity'])
    await tabs[1]!.trigger('click')
    expect(visibleProviders(app)).toEqual(['antigravity'])
    expect(tabs[1]!.attributes('aria-selected')).toBe('true')
    await tabs[0]!.trigger('click')
    expect(visibleProviders(app)).toEqual(['codex', 'claude'])
  })

  it('returns to the main tab when the popup is opened again', async () => {
    const app = render()
    await flushPromises()
    await app.findAll('[role="tab"]')[1]!.trigger('click')
    const shown = vi.mocked(listen).mock.calls.find(([event]) => event === 'popup://shown')?.[1]
    shown?.({ event: 'popup://shown', id: 0, payload: undefined })
    await nextTick()
    expect(visibleProviders(app)).toEqual(['codex', 'claude'])
  })

  it('moves focus and selection with the tab keyboard controls', async () => {
    const app = render()
    const tabs = app.findAll('[role="tab"]')
    await tabs[0]!.trigger('keydown', { key: 'ArrowRight' })
    expect(visibleProviders(app)).toEqual(['antigravity'])
    expect(document.activeElement).toBe(tabs[1]!.element)
    await tabs[1]!.trigger('keydown', { key: 'Home' })
    expect(visibleProviders(app)).toEqual(['codex', 'claude'])
  })

  it('filters disabled subscriptions and falls back when the current tab is disabled', async () => {
    savedSettings.monitoring.claude = false
    const app = render()
    expect(visibleProviders(app)).toEqual(['codex'])
    await app.findAll('[role="tab"]')[1]!.trigger('click')
    savedSettings.monitoring.antigravity = false
    await nextTick()
    expect(visibleProviders(app)).toEqual(['codex'])
    expect(app.find('[role="tablist"]').exists()).toBe(false)
    savedSettings.monitoring.antigravity = true
    await nextTick()
    expect(visibleProviders(app)).toEqual(['codex'])
  })

  it('supports an Antigravity-only subscription and an empty selection', async () => {
    savedSettings.monitoring.codex = false
    savedSettings.monitoring.claude = false
    const app = render()
    expect(visibleProviders(app)).toEqual(['antigravity'])
    savedSettings.monitoring.antigravity = false
    await nextTick()
    expect(visibleProviders(app)).toEqual([])
    expect(app.text()).toContain('No providers enabled')
    await app.findAll('button').find((button) => button.text() === 'Choose providers in Settings')!.trigger('click')
    expect(app.findComponent({ name: 'SettingsPanel' }).exists()).toBe(true)
  })
})
