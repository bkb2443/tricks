import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ChatPanel from './ChatPanel.vue'
import type { ChatMessage } from './ChatPanel.vue'

const msgs: ChatMessage[] = [
  { from: 'Alice', text: 'Hello', timestamp: 1 },
  { from: 'Bob',   text: 'Hi!',   timestamp: 2 },
]

describe('ChatPanel', () => {
  it('renders list of messages', () => {
    const w = mount(ChatPanel, { props: { messages: msgs, onSend: vi.fn() } })
    expect(w.text()).toContain('Alice:')
    expect(w.text()).toContain('Hello')
    expect(w.text()).toContain('Bob:')
    expect(w.text()).toContain('Hi!')
  })

  it('shows empty-state copy when no messages', () => {
    const w = mount(ChatPanel, { props: { messages: [], onSend: vi.fn() } })
    expect(w.text()).toContain('No messages yet')
    expect(w.find('.chat-empty').exists()).toBe(true)
  })

  it('fires onSend with trimmed text on Enter', async () => {
    const onSend = vi.fn()
    const w = mount(ChatPanel, { props: { messages: [], onSend } })
    const input = w.find('input')
    await input.setValue('  hello world  ')
    await input.trigger('keydown.enter')
    expect(onSend).toHaveBeenCalledOnce()
    expect(onSend).toHaveBeenCalledWith('hello world')
  })

  it('fires onSend with trimmed text on Send button click', async () => {
    const onSend = vi.fn()
    const w = mount(ChatPanel, { props: { messages: [], onSend } })
    await w.find('input').setValue('  test  ')
    await w.find('.send-btn').trigger('click')
    expect(onSend).toHaveBeenCalledWith('test')
  })

  it('does not fire onSend for blank input', async () => {
    const onSend = vi.fn()
    const w = mount(ChatPanel, { props: { messages: [], onSend } })
    await w.find('input').setValue('   ')
    await w.find('.send-btn').trigger('click')
    await w.find('input').trigger('keydown.enter')
    expect(onSend).not.toHaveBeenCalled()
  })

  it('clears the input after sending', async () => {
    const w = mount(ChatPanel, { props: { messages: [], onSend: vi.fn() } })
    const input = w.find('input')
    await input.setValue('hello')
    await w.find('.send-btn').trigger('click')
    expect((input.element as HTMLInputElement).value).toBe('')
  })

  it('Send button is disabled when input is blank', () => {
    const w = mount(ChatPanel, { props: { messages: [], onSend: vi.fn() } })
    expect(w.find('.send-btn').attributes('disabled')).toBeDefined()
  })

  it('system messages get the system CSS class', () => {
    const systemMsgs: ChatMessage[] = [{ from: 'System', text: 'Game started', timestamp: 0 }]
    const w = mount(ChatPanel, { props: { messages: systemMsgs, onSend: vi.fn() } })
    expect(w.find('.chat-msg.system').exists()).toBe(true)
  })
})
