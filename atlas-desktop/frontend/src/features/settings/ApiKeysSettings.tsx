import React, { useEffect, useState } from 'react';
import { CheckCircle2, Cpu, ExternalLink, KeyRound, Server } from 'lucide-react';
import { api } from '../../services/api';
import type { ChatConfig } from '../../types';

type Provider = 'openai' | 'anthropic' | 'compatible';
type ProviderForm = { apiKey: string; endpointUrl: string; model: string };

const providers: { id: Provider; name: string; description: string; link: string }[] = [
  { id: 'openai', name: 'OpenAI', description: 'Responses API', link: 'https://platform.openai.com/api-keys' },
  { id: 'anthropic', name: 'Anthropic', description: 'Messages API', link: 'https://platform.claude.com/settings/keys' },
  { id: 'compatible', name: 'OpenAI-compatible', description: 'DeepSeek, Ollama, atau endpoint custom', link: 'https://platform.deepseek.com/api_keys' },
];

const initialForms: Record<Provider, ProviderForm> = {
  openai: { apiKey: '', endpointUrl: '', model: '' },
  anthropic: { apiKey: '', endpointUrl: '', model: '' },
  compatible: { apiKey: '', endpointUrl: '', model: '' },
};

function formsFromConfig(config: ChatConfig, previous: Record<Provider, ProviderForm>): Record<Provider, ProviderForm> {
  return {
    openai: { apiKey: previous.openai.apiKey, endpointUrl: config.openai_endpoint_url, model: config.openai_model },
    anthropic: { apiKey: previous.anthropic.apiKey, endpointUrl: config.anthropic_endpoint_url, model: config.anthropic_model },
    compatible: { apiKey: previous.compatible.apiKey, endpointUrl: config.compatible_endpoint_url, model: config.compatible_model },
  };
}

export const ApiKeysSettings: React.FC = () => {
  const [config, setConfig] = useState<ChatConfig | null>(null);
  const [forms, setForms] = useState<Record<Provider, ProviderForm>>(initialForms);
  const [pending, setPending] = useState<Provider | null>(null);
  const [notice, setNotice] = useState('');
  const [error, setError] = useState('');

  const refresh = async () => {
    const next = await api.getChatConfig();
    setConfig(next);
    setForms((previous) => formsFromConfig(next, previous));
  };

  useEffect(() => { refresh().catch((err: Error) => setError(err.message)); }, []);

  const updateField = (provider: Provider, field: keyof ProviderForm, value: string) => {
    setForms((previous) => ({
      ...previous,
      [provider]: { ...previous[provider], [field]: value },
    }));
  };

  const save = async (event: React.FormEvent, provider: Provider) => {
    event.preventDefault();
    const form = forms[provider];
    if (!form.endpointUrl.trim() || !form.model.trim()) return;
    setPending(provider);
    setNotice('');
    setError('');
    try {
      await api.saveChatConfig({
        provider,
        apiKey: form.apiKey.trim(),
        endpointUrl: form.endpointUrl.trim(),
        model: form.model.trim(),
      });
      setForms((previous) => ({ ...previous, [provider]: { ...previous[provider], apiKey: '' } }));
      await refresh();
      setNotice(`${providers.find((item) => item.id === provider)?.name} berhasil diperbarui.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Konfigurasi LLM gagal disimpan.');
    } finally {
      setPending(null);
    }
  };

  const remove = async (provider: Provider) => {
    setPending(provider);
    setNotice('');
    setError('');
    try {
      await api.deleteChatKey(provider);
      await refresh();
      setNotice(`${providers.find((item) => item.id === provider)?.name} API key dihapus.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'API key gagal dihapus.');
    } finally {
      setPending(null);
    }
  };

  return (
    <section id="api-keys" className="max-w-4xl space-y-4">
      <div className="flex items-start gap-3">
        <div className="mt-0.5 grid h-9 w-9 shrink-0 place-items-center rounded-lg border border-indigo-200 bg-indigo-50 text-indigo-700 dark:border-indigo-500/30 dark:bg-indigo-500/10 dark:text-indigo-300">
          <Cpu className="h-4 w-4" />
        </div>
        <div>
          <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">Konfigurasi LLM</h3>
          <p className="mt-1 max-w-2xl text-xs leading-5 text-slate-500 dark:text-zinc-400">
            Endpoint dan model diatur di sini. Chat hanya memilih provider aktif. API key disimpan lokal dan tidak pernah ditampilkan kembali.
          </p>
        </div>
      </div>

      <div className="grid gap-4 xl:grid-cols-3">
        {providers.map(({ id, name, description, link }) => {
          const source = config?.[id === 'openai' ? 'openai_source' : id === 'anthropic' ? 'anthropic_source' : 'compatible_source'];
          const form = forms[id];
          const isPending = pending === id;
          return (
            <form key={id} onSubmit={(event) => save(event, id)} className="flex min-w-0 flex-col rounded-xl border border-slate-200 bg-white p-4 shadow-xs dark:border-zinc-800 dark:bg-zinc-950">
              <div className="flex items-start justify-between gap-3 border-b border-slate-100 pb-3 dark:border-zinc-800">
                <div className="min-w-0">
                  <h4 className="text-sm font-semibold text-slate-900 dark:text-zinc-100">{name}</h4>
                  <p className="mt-0.5 text-[11px] text-slate-500 dark:text-zinc-400">{description}</p>
                </div>
                <span className={`inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-[10px] font-semibold ${source ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-500/10 dark:text-emerald-300' : 'bg-slate-100 text-slate-500 dark:bg-zinc-900 dark:text-zinc-400'}`}>
                  {source && <CheckCircle2 className="h-3 w-3" />}{source === 'saved' ? 'Tersimpan' : source === 'environment' ? 'Environment' : 'Belum siap'}
                </span>
              </div>

              <div className="mt-4 space-y-3">
                <label className="block text-xs font-medium text-slate-700 dark:text-zinc-300">
                  <span className="mb-1.5 flex items-center gap-1.5"><Server className="h-3.5 w-3.5" /> Endpoint URL</span>
                  <input type="url" required value={form.endpointUrl} onChange={(event) => updateField(id, 'endpointUrl', event.target.value)} aria-label={`${name} endpoint URL`} className="h-11 w-full rounded-lg border border-slate-200 bg-slate-50 px-3 font-mono text-[11px] text-slate-900 outline-2 outline-transparent focus:border-indigo-500 focus:outline-indigo-500 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100" />
                </label>
                <label className="block text-xs font-medium text-slate-700 dark:text-zinc-300">
                  <span className="mb-1.5 flex items-center gap-1.5"><Cpu className="h-3.5 w-3.5" /> Model</span>
                  <input required value={form.model} onChange={(event) => updateField(id, 'model', event.target.value)} aria-label={`${name} model`} className="h-11 w-full rounded-lg border border-slate-200 bg-slate-50 px-3 font-mono text-xs text-slate-900 outline-2 outline-transparent focus:border-indigo-500 focus:outline-indigo-500 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100" />
                </label>
                <label className="block text-xs font-medium text-slate-700 dark:text-zinc-300">
                  <span className="mb-1.5 flex items-center gap-1.5"><KeyRound className="h-3.5 w-3.5" /> API key</span>
                  <input type="password" autoComplete="new-password" value={form.apiKey} onChange={(event) => updateField(id, 'apiKey', event.target.value)} placeholder={source ? 'Kosongkan untuk mempertahankan key' : 'Tempel API key'} aria-label={`${name} API key`} className="h-11 w-full rounded-lg border border-slate-200 bg-slate-50 px-3 text-xs text-slate-900 outline-2 outline-transparent focus:border-indigo-500 focus:outline-indigo-500 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100" />
                </label>
              </div>

              <div className="mt-auto flex items-center gap-2 pt-4">
                <button type="submit" disabled={!form.endpointUrl.trim() || !form.model.trim() || pending !== null} className="h-11 whitespace-nowrap rounded-lg bg-indigo-600 px-3 text-xs font-semibold text-white hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500 disabled:cursor-not-allowed disabled:opacity-50">
                  {isPending ? 'Menyimpan…' : 'Simpan'}
                </button>
                {source === 'saved' && <button type="button" onClick={() => remove(id)} disabled={pending !== null} className="h-11 whitespace-nowrap rounded-lg border border-rose-200 px-3 text-xs font-medium text-rose-600 hover:bg-rose-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-rose-500 disabled:cursor-not-allowed disabled:opacity-50 dark:border-rose-900 dark:text-rose-400 dark:hover:bg-rose-950/40">Hapus key</button>}
                <a href={link} target="_blank" rel="noopener noreferrer" className="ml-auto inline-flex h-11 items-center gap-1 whitespace-nowrap text-[11px] font-medium text-indigo-600 hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500 dark:text-indigo-400">Buat key <ExternalLink className="h-3 w-3" /></a>
              </div>
            </form>
          );
        })}
      </div>

      <div className="min-h-5" aria-live="polite">
        {notice && <p className="text-xs text-emerald-600 dark:text-emerald-400">{notice}</p>}
        {error && <p role="alert" className="text-xs text-rose-600 dark:text-rose-400">{error}</p>}
      </div>
    </section>
  );
};
