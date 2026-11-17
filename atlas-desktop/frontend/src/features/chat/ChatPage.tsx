/* Hallmark · pre-emit critique: P4 H5 E4 S5 R5 V4 */
import React, { useEffect, useRef, useState } from 'react';
import {
  ArrowUp,
  Bot,
  Database,
  ExternalLink,
  MessageSquare,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  Settings2,
  Trash2,
} from 'lucide-react';
import { api } from '../../services/api';
import type { ChatConfig, ChatSource } from '../../types';
import { MarkdownMessage } from './MarkdownMessage';
import './chat.css';

type Provider = 'compatible' | 'openai' | 'anthropic';
type Message = { role: 'user' | 'assistant'; content: string; sources?: ChatSource[] };
type Conversation = { id: string; title: string; messages: Message[]; updatedAt: number };

const STORAGE_KEY = 'atlas-chat-conversations-v1';
const PROVIDER_KEY = 'atlas-chat-provider-v1';

const providerName: Record<Provider, string> = {
  compatible: 'OpenAI-compatible',
  openai: 'OpenAI',
  anthropic: 'Anthropic',
};

const starters = [
  { label: 'Ringkas arsitektur', prompt: 'Ringkas arsitektur sistem berdasarkan artefak yang tersedia.' },
  { label: 'Telusuri autentikasi', prompt: 'Cari tiket dan perubahan kode yang berkaitan dengan autentikasi.' },
  { label: 'Lihat perubahan terbaru', prompt: 'Apa perubahan terbaru yang paling penting di knowledge Atlas?' },
];

function readConversations(): Conversation[] {
  try {
    const value = JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]');
    if (!Array.isArray(value)) return [];
    return value.filter((item): item is Conversation =>
      typeof item?.id === 'string'
      && typeof item?.title === 'string'
      && typeof item?.updatedAt === 'number'
      && Array.isArray(item?.messages)
      && item.messages.every((message: Message) =>
        (message?.role === 'user' || message?.role === 'assistant') && typeof message?.content === 'string'))
      .slice(0, 30);
  } catch {
    return [];
  }
}

function readProvider(): Provider {
  const saved = localStorage.getItem(PROVIDER_KEY);
  return saved === 'openai' || saved === 'anthropic' || saved === 'compatible' ? saved : 'compatible';
}

export const ChatPage: React.FC<{ onConfigure: () => void }> = ({ onConfigure }) => {
  const [conversations, setConversations] = useState<Conversation[]>(readConversations);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [historyOpen, setHistoryOpen] = useState(() => window.innerWidth > 900);
  const [input, setInput] = useState('');
  const [provider, setProvider] = useState<Provider>(readProvider);
  const [config, setConfig] = useState<ChatConfig | null>(null);
  const [error, setError] = useState('');
  const [pendingId, setPendingId] = useState<string | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const active = conversations.find((conversation) => conversation.id === activeId);
  const messages = active?.messages || [];
  const isPending = pendingId !== null;
  const isProviderReady = Boolean(config?.[provider]);

  useEffect(() => {
    api.getChatConfig().then((data) => {
      setConfig(data);
      if (!data[provider]) {
        if (data.compatible) setProvider('compatible');
        else if (data.openai) setProvider('openai');
        else if (data.anthropic) setProvider('anthropic');
      }
    }).catch((err: Error) => setError(err.message));
  }, []);

  useEffect(() => {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(conversations.slice(0, 30))); } catch { /* Storage can be disabled. */ }
  }, [conversations]);

  useEffect(() => {
    try { localStorage.setItem(PROVIDER_KEY, provider); } catch { /* Storage can be disabled. */ }
  }, [provider]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
  }, [activeId, messages.length, pendingId]);

  const chooseProvider = (next: Provider) => {
    setProvider(next);
    setError('');
  };

  const newChat = () => {
    if (isPending) return;
    setActiveId(null);
    setInput('');
    setError('');
    inputRef.current?.focus();
  };

  const selectConversation = (id: string) => {
    setActiveId(id);
    setError('');
    if (window.innerWidth <= 900) setHistoryOpen(false);
  };

  const sendMessage = async (raw: string) => {
    const question = raw.trim();
    if (!question || isPending || !isProviderReady) return;
    const id = activeId || crypto.randomUUID();
    const history = messages.map(({ role, content }) => ({ role, content }));
    setError('');
    setInput('');
    setActiveId(id);
    setPendingId(id);
    setConversations((previous) => {
      const existing = previous.find((item) => item.id === id);
      const next: Conversation = existing
        ? { ...existing, messages: [...existing.messages, { role: 'user', content: question }], updatedAt: Date.now() }
        : { id, title: question.slice(0, 52), messages: [{ role: 'user', content: question }], updatedAt: Date.now() };
      return [next, ...previous.filter((item) => item.id !== id)].slice(0, 30);
    });
    try {
      const result = await api.sendChat({ message: question, provider, history });
      setConversations((previous) => previous.map((item) => item.id === id
        ? { ...item, messages: [...item.messages, { role: 'assistant', content: result.answer, sources: result.sources }], updatedAt: Date.now() }
        : item));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Permintaan chat gagal.');
      setInput(question);
      setConversations((previous) => previous.map((item) => item.id === id
        ? { ...item, messages: item.messages.slice(0, -1) }
        : item).filter((item) => item.messages.length > 0));
    } finally {
      setPendingId(null);
    }
  };

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    void sendMessage(input);
  };

  const removeChat = (id: string) => {
    if (isPending) return;
    setConversations((previous) => previous.filter((item) => item.id !== id));
    if (activeId === id) setActiveId(null);
  };

  return (
    <div className="chat-shell">
      <aside className={`chat-history ${historyOpen ? '' : 'chat-history--closed'}`} aria-label="Riwayat percakapan">
        <div className="chat-history-head">
          <div>
            <span className="chat-kicker">Percakapan</span>
            <strong>Riwayat lokal</strong>
          </div>
          <button type="button" className="chat-icon-button" onClick={() => setHistoryOpen(false)} title="Tutup riwayat" aria-label="Tutup riwayat"><PanelLeftClose size={17} /></button>
        </div>
        <div className="chat-history-new-wrap">
          <button type="button" className="chat-history-new" onClick={newChat} disabled={isPending}><Plus size={16} /> Chat baru</button>
        </div>
        <div className="chat-history-list">
          {conversations.length === 0 && <div className="chat-history-empty"><MessageSquare size={18} /><p>Belum ada percakapan. Mulai dari pertanyaan pertama.</p></div>}
          {conversations.map((item) => (
            <div key={item.id} className={`chat-history-row ${activeId === item.id ? 'is-active' : ''}`}>
              <button type="button" className="chat-history-select" onClick={() => selectConversation(item.id)} disabled={isPending} title={item.title}>
                <MessageSquare size={14} /><span>{item.title}</span>
              </button>
              <button type="button" className="chat-history-delete" onClick={() => removeChat(item.id)} disabled={isPending} aria-label={`Hapus percakapan ${item.title}`} title="Hapus percakapan"><Trash2 size={14} /></button>
            </div>
          ))}
        </div>
        <button type="button" className="chat-history-settings" onClick={onConfigure}><Settings2 size={16} /><span>Konfigurasi LLM</span></button>
      </aside>

      {historyOpen && <button type="button" className="chat-history-backdrop" onClick={() => setHistoryOpen(false)} aria-label="Tutup riwayat" />}

      <section className="chat-main" aria-label="Atlas Chat">
        <header className="chat-toolbar">
          <div className="chat-toolbar-left">
            <button type="button" className="chat-icon-button" onClick={() => setHistoryOpen((value) => !value)} title={historyOpen ? 'Sembunyikan riwayat' : 'Tampilkan riwayat'} aria-label={historyOpen ? 'Sembunyikan riwayat' : 'Tampilkan riwayat'}>{historyOpen ? <PanelLeftClose size={18} /> : <PanelLeftOpen size={18} />}</button>
            <div className="chat-toolbar-title"><strong>Atlas Chat</strong><span>Jawaban berbasis knowledge terindeks</span></div>
          </div>
          <div className="chat-toolbar-actions">
            <label className={`chat-provider ${isProviderReady ? 'is-ready' : 'is-missing'}`}>
              <span className="chat-provider-dot" aria-hidden="true" />
              <select value={provider} onChange={(event) => chooseProvider(event.target.value as Provider)} aria-label="Provider LLM" disabled={isPending}>
                <option value="compatible">OpenAI-compatible</option>
                <option value="openai">OpenAI</option>
                <option value="anthropic">Anthropic</option>
              </select>
            </label>
            <button type="button" className="chat-icon-button" onClick={onConfigure} title="Konfigurasi LLM" aria-label="Konfigurasi LLM"><Settings2 size={17} /></button>
            <button type="button" className="chat-icon-button chat-new-mobile" onClick={newChat} disabled={isPending} title="Chat baru" aria-label="Chat baru"><Plus size={18} /></button>
          </div>
        </header>

        <div className="chat-scroll" aria-live="polite">
          {messages.length === 0 ? (
            <div className="chat-welcome">
              <div className="chat-welcome-copy">
                <div className="chat-welcome-label"><Database size={14} /> Atlas knowledge workspace</div>
                <h2>Tanya Atlas.<br />Telusuri sumbernya.</h2>
                <p>Gunakan tiket, dokumentasi, repositori, dan artefak yang sudah tersinkron untuk menemukan jawaban yang bisa diperiksa kembali.</p>
              </div>
              <div className="chat-starters" aria-label="Contoh pertanyaan">
                {starters.map((starter, index) => (
                  <button key={starter.label} type="button" onClick={() => { setInput(starter.prompt); inputRef.current?.focus(); }}>
                    <span>{String(index + 1).padStart(2, '0')}</span><strong>{starter.label}</strong><ArrowUp size={15} />
                  </button>
                ))}
              </div>
              {!isProviderReady && (
                <div className="chat-setup-callout" role="status">
                  <div><strong>{providerName[provider]} belum siap</strong><span>Lengkapi endpoint, model, dan API key di konfigurasi LLM.</span></div>
                  <button type="button" onClick={onConfigure}>Buka konfigurasi</button>
                </div>
              )}
            </div>
          ) : (
            <div className="chat-messages">
              {messages.map((message, index) => (
                <article key={`${activeId}-${index}`} className={`chat-message chat-message--${message.role}`}>
                  {message.role === 'assistant' && <div className="chat-avatar" aria-hidden="true"><Bot size={17} /></div>}
                  <div className="chat-message-body">
                    {message.role === 'assistant' && <span className="chat-message-author">Atlas</span>}
                    {message.role === 'assistant'
                      ? <MarkdownMessage content={message.content} />
                      : <div className="chat-message-text">{message.content}</div>}
                    {message.sources && message.sources.length > 0 && (
                      <details className="chat-sources" open>
                        <summary>{message.sources.length} sumber digunakan</summary>
                        <div className="chat-source-list">
                          {message.sources.map((source, sourceIndex) => (
                            <div key={source.id} className="chat-source">
                              <span className="chat-source-index">{sourceIndex + 1}</span>
                              <span className="chat-source-title">
                                {/^(https?):\/\//i.test(source.url)
                                  ? <a href={source.url} target="_blank" rel="noopener noreferrer">{source.title}<ExternalLink size={12} /></a>
                                  : source.title}
                                <small>{source.kind}</small>
                              </span>
                            </div>
                          ))}
                        </div>
                      </details>
                    )}
                  </div>
                </article>
              ))}
              {pendingId === activeId && (
                <div className="chat-message chat-message--assistant" role="status" aria-label="Atlas sedang menyusun jawaban">
                  <div className="chat-avatar" aria-hidden="true"><Bot size={17} /></div>
                  <div className="chat-thinking"><span /><span /><span /><em>Menelusuri knowledge…</em></div>
                </div>
              )}
              <div ref={bottomRef} />
            </div>
          )}
        </div>

        <footer className="chat-composer-wrap">
          <div className="chat-composer-inner">
            {error && <div role="alert" className="chat-error"><span>{error}</span><button type="button" onClick={() => setError('')} aria-label="Tutup pesan error">Tutup</button></div>}
            {!isProviderReady && messages.length > 0 && <div className="chat-provider-warning">Provider belum dikonfigurasi. <button type="button" onClick={onConfigure}>Atur sekarang</button></div>}
            <form onSubmit={submit} className="chat-composer">
              <textarea ref={inputRef} rows={2} value={input} onChange={(event) => setInput(event.target.value)} onKeyDown={(event) => {
                if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); void sendMessage(input); }
              }} maxLength={2000} placeholder="Tanyakan sesuatu dari knowledge Atlas…" aria-label="Pesan chat" disabled={isPending} />
              <button type="submit" disabled={isPending || !input.trim() || !isProviderReady} aria-label="Kirim pesan" title={isProviderReady ? 'Kirim pesan' : 'Konfigurasikan provider terlebih dahulu'}><ArrowUp size={19} /></button>
            </form>
            <div className="chat-composer-meta"><span>{providerName[provider]}</span><span>Enter untuk kirim · Shift + Enter untuk baris baru</span><span>{input.length}/2000</span></div>
          </div>
        </footer>
      </section>
    </div>
  );
};
