import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import ChatPanel from './ChatPanel.vue'
import type { ChatMessage } from '@/engine/types'

function msg(from: string, text: string, timestamp = 0): ChatMessage {
  return { from, text, timestamp }
}

describe('ChatPanel', () => {
  it('renders a list of messages showing from name and text', () => {
    const messages = [
      msg('Alice', 'hello there'),
      msg('System', 'game started'),
    ]
    const w = mount(ChatPanel, { props: { messages } })
    expect(w.text()).toContain('Alice')
    expect(w.text()).toContain('hello there')
    expect(w.text()).toContain('System')
    expect(w.text()).toContain('game started')
  })

  it('shows "No messages yet…" empty state when messages is empty', () => {
    const w = mount(ChatPanel, { props: { messages: [] } })
    expect(w.text()).toContain('No messages yet…')
  })

  it('emits send with trimmed text when Enter is pressed', async () => {
    const w = mount(ChatPanel, { props: { messages: [] } })
    const input = w.find('input')
    await input.setValue('  hello  ')
    await input.trigger('keydown.enter')
    expect(w.emitted('send')).toBeTruthy()
    expect(w.emitted('send')![0]).toEqual(['hello'])
  })

  it('emits send with trimmed text when Send button is clicked', async () => {
    const w = mount(ChatPanel, { props: { messages: [] } })
    const input = w.find('input')
    await input.setValue('  world  ')
    await w.find('button').trigger('click')
    expect(w.emitted('send')).toBeTruthy()
    expect(w.emitted('send')![0]).toEqual(['world'])
  })

  it('does not emit send for blank input on Enter', async () => {
    const w = mount(ChatPanel, { props: { messages: [] } })
    const input = w.find('input')
    await input.setValue('   ')
    await input.trigger('keydown.enter')
    expect(w.emitted('send')).toBeFalsy()
  })

  it('does not emit send for blank input on Send button click', async () => {
    const w = mount(ChatPanel, { props: { messages: [] } })
    const input = w.find('input')
    await input.setValue('')
    await w.find('button').trigger('click')
    expect(w.emitted('send')).toBeFalsy()
  })

  it('clears the input after a successful send', async () => {
    const w = mount(ChatPanel, { props: { messages: [] } })
    const input = w.find('input')
    await input.setValue('test message')
    await input.trigger('keydown.enter')
    expect((input.element as HTMLInputElement).value).toBe('')
  })
})
