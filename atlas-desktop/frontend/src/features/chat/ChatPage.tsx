import React, { useState } from 'react';
import { MessageSquare, Send, Sparkles, Database, Bot } from 'lucide-react';

export const ChatPage: React.FC = () => {
  const [messages, setMessages] = useState<Array<{ sender: 'user' | 'atlas'; text: string }>>([
    {
      sender: 'atlas',
      text: 'Hello! I am your Atlas Local Context AI Assistant. Ask any engineering question, and I will retrieve matching local knowledge context from Atlas SQLite FTS5.',
    },
  ]);
  const [input, setInput] = useState('');

  const handleSend = (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;

    const userText = input;
    setMessages((prev) => [...prev, { sender: 'user', text: userText }]);
    setInput('');

    setTimeout(() => {
      setMessages((prev) => [
        ...prev,
        {
          sender: 'atlas',
          text: `Retrieved context for "${userText}": Found matching tickets and specs in local Atlas SQLite DB. (Local LLM Provider Integration Active).`,
        },
      ]);
    }, 600);
  };

  return (
    <div className="p-6 space-y-6 flex flex-col h-[calc(100vh-3.5rem)] max-w-7xl mx-auto w-full">
      <div>
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-bold text-slate-900 dark:text-zinc-100 tracking-tight">AI Knowledge Assistant</h2>
          <span className="text-[10px] px-2 py-0.5 rounded bg-indigo-50 dark:bg-indigo-950/60 border border-indigo-200 dark:border-indigo-800/40 text-indigo-700 dark:text-indigo-300 font-mono font-medium">
            Feature Flag Enabled
          </span>
        </div>
        <p className="text-xs text-slate-500 dark:text-zinc-400 mt-1">
          Chat interface powered by Atlas as the local RAG context provider (Ollama / Claude / OpenAI).
        </p>
      </div>

      {/* Chat Messages */}
      <div className="flex-1 glass-card p-4 rounded-xl border border-slate-200 dark:border-zinc-800 overflow-y-auto space-y-4 text-xs shadow-xs">
        {messages.map((m, idx) => (
          <div
            key={idx}
            className={`flex gap-3 ${m.sender === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            {m.sender === 'atlas' && (
              <div className="w-7 h-7 rounded-lg bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 flex items-center justify-center shrink-0 border border-indigo-200 dark:border-indigo-500/30">
                <Bot className="w-4 h-4" />
              </div>
            )}
            <div
              className={`p-3.5 rounded-xl max-w-md ${
                m.sender === 'user'
                  ? 'bg-indigo-600 text-white font-medium shadow-xs'
                  : 'bg-slate-100 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 text-slate-800 dark:text-zinc-200'
              }`}
            >
              {m.text}
            </div>
          </div>
        ))}
      </div>

      {/* Input */}
      <form onSubmit={handleSend} className="flex gap-2">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder='Ask about engineering knowledge e.g. "Explain why payment retry exists"...'
          className="flex-1 bg-white dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg px-4 py-2.5 text-xs text-slate-900 dark:text-zinc-100 placeholder-slate-400 dark:placeholder-zinc-500 focus:outline-none focus:border-indigo-500 shadow-xs"
        />
        <button
          type="submit"
          className="px-4 py-2.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium transition flex items-center gap-1.5 shadow-xs"
        >
          <Send className="w-3.5 h-3.5" />
          <span>Send</span>
        </button>
      </form>
    </div>
  );
};
