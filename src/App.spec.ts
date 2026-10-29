import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import App from './App.vue'
import { invoke } from './lib/tauri'

vi.mock('./lib/tauri', () => ({
  isTauri: true,
  invoke: vi.fn(),
  listen: vi.fn(async () => vi.fn()),
}))

vi.mock('./stores/usage', () => ({
  useUsageStore: () => ({
    codex: {}, claude: {}, now: 0, refreshing: false, lastError: null,
    initialize: vi.fn(), refreshAll: vi.fn(), dispose: vi.fn(),
  }),
}))

vi.mock('./stores/settings', () => ({
  useSettingsStore: () => ({ initialize: vi.fn() }),
}))

let wrapper: VueWrapper | undefined

beforeEach(() => {
  vi.clearAllMocks()
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
