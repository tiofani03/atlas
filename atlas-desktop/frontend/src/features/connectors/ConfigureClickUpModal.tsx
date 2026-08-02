import React, { useState } from 'react';
import { X, Check, AlertCircle, Loader2 } from 'lucide-react';
import { api } from '../../services/api';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  initialConfig?: {
    id: string;
    instance_url: string;
    workspaces: string[];
    spaces: string[];
    folders: string[];
    lists: string[];
  };
}

const splitCsv = (value: string) =>
  value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);

export const ConfigureClickUpModal: React.FC<ModalProps> = ({ isOpen, onClose, onSuccess, initialConfig }) => {
  const [id, setId] = useState(initialConfig?.id || 'clickup-main');
  const [instanceUrl, setInstanceUrl] = useState(initialConfig?.instance_url || 'https://api.clickup.com/api/v2');
  const [apiToken, setApiToken] = useState('');
  const [workspaces, setWorkspaces] = useState(initialConfig?.workspaces?.join(', ') || '');
  const [spaces, setSpaces] = useState(initialConfig?.spaces?.join(', ') || '');
  const [folders, setFolders] = useState(initialConfig?.folders?.join(', ') || '');
  const [lists, setLists] = useState(initialConfig?.lists?.join(', ') || '');
  const [isValidating, setIsValidating] = useState(false);
  const [validationResult, setValidationResult] = useState<{ valid: boolean; message: string } | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleValidate = async () => {
    setIsValidating(true);
    setValidationResult(null);
    try {
      const res = await api.validateCredentials({
        provider: 'clickup',
        instance_url: instanceUrl,
        email: '',
        api_token: apiToken,
      });
      setValidationResult(res);
    } catch (err: unknown) {
      setValidationResult({ valid: false, message: (err as Error).message || 'Validation failed' });
    } finally {
      setIsValidating(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      await api.saveClickUpConnector({
        id,
        instance_url: instanceUrl,
        api_token: apiToken || undefined,
        workspaces: splitCsv(workspaces),
        spaces: splitCsv(spaces),
        folders: splitCsv(folders),
        lists: splitCsv(lists),
      });
      onSuccess();
      onClose();
    } catch (err: unknown) {
      setValidationResult({ valid: false, message: (err as Error).message || 'Failed to save configuration' });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-900/40 dark:bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-xl w-full max-w-md p-6 space-y-4 shadow-2xl">
        <div className="flex items-center justify-between border-b border-slate-200 dark:border-zinc-800 pb-3">
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 rounded bg-pink-50 dark:bg-pink-500/20 text-pink-600 dark:text-pink-400 flex items-center justify-center font-bold text-[10px] border border-pink-200 dark:border-pink-500/30">
              CU
            </div>
            <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">Configure ClickUp Connector</h3>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 dark:text-zinc-400 dark:hover:text-zinc-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-3 text-xs">
          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Connector ID</label>
            <input
              type="text"
              value={id}
              onChange={(e) => setId(e.target.value)}
              required
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
            />
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">API Base URL</label>
            <input
              type="url"
              value={instanceUrl}
              onChange={(e) => setInstanceUrl(e.target.value)}
              placeholder="https://api.clickup.com/api/v2"
              required
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
            />
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">
              Personal API Token {initialConfig && <span className="text-[10px] text-emerald-600 dark:text-emerald-400 font-normal">(Saved - leave empty to keep unchanged)</span>}
            </label>
            <input
              type="password"
              value={apiToken}
              onChange={(e) => setApiToken(e.target.value)}
              placeholder={initialConfig ? 'Token saved' : 'pk_...'}
              required={!initialConfig}
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500"
            />
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Workspace / Team IDs</label>
            <input
              type="text"
              value={workspaces}
              onChange={(e) => setWorkspaces(e.target.value)}
              placeholder="Optional - leave empty to sync all authorized workspaces"
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
            />
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
            <div>
              <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Space IDs</label>
              <input
                type="text"
                value={spaces}
                onChange={(e) => setSpaces(e.target.value)}
                placeholder="Optional"
                className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
              />
            </div>
            <div>
              <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Folder IDs</label>
              <input
                type="text"
                value={folders}
                onChange={(e) => setFolders(e.target.value)}
                placeholder="Optional"
                className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
              />
            </div>
            <div>
              <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">List IDs</label>
              <input
                type="text"
                value={lists}
                onChange={(e) => setLists(e.target.value)}
                placeholder="Optional"
                className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
              />
            </div>
          </div>

          {validationResult && (
            <div
              className={`p-2.5 rounded text-xs flex items-center gap-2 ${
                validationResult.valid
                  ? 'bg-emerald-50 dark:bg-emerald-950/50 border border-emerald-200 dark:border-emerald-800/50 text-emerald-700 dark:text-emerald-400'
                  : 'bg-rose-50 dark:bg-rose-950/50 border border-rose-200 dark:border-rose-800/50 text-rose-700 dark:text-rose-400'
              }`}
            >
              {validationResult.valid ? (
                <Check className="w-4 h-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
              ) : (
                <AlertCircle className="w-4 h-4 shrink-0 text-rose-600 dark:text-rose-400" />
              )}
              <span>{validationResult.message}</span>
            </div>
          )}

          <div className="flex items-center justify-between border-t border-slate-200 dark:border-zinc-800 pt-3 mt-4">
            <button
              type="button"
              onClick={handleValidate}
              disabled={isValidating || !apiToken}
              className="px-3 py-1.5 rounded bg-slate-100 dark:bg-zinc-800 hover:bg-slate-200 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-300 font-medium transition disabled:opacity-50 flex items-center gap-1.5"
            >
              {isValidating && <Loader2 className="w-3 h-3 animate-spin" />}
              <span>Validate</span>
            </button>

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-3 py-1.5 rounded bg-slate-100 dark:bg-zinc-800 hover:bg-slate-200 dark:hover:bg-zinc-700 text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200 transition"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-1.5 rounded bg-indigo-600 hover:bg-indigo-500 text-white font-bold transition disabled:opacity-50"
              >
                Save Connector
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};
