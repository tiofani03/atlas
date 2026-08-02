import React, { useState } from 'react';
import { X, Check, AlertCircle, Loader2 } from 'lucide-react';
import { api } from '../../services/api';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  initialConfig?: {
    id: string;
    workspace?: string;
    projects?: string[];
  };
}

export const ConfigureAsanaModal: React.FC<ModalProps> = ({ isOpen, onClose, onSuccess, initialConfig }) => {
  const [id, setId] = useState(initialConfig?.id || 'asana-main');
  const [workspace, setWorkspace] = useState(initialConfig?.workspace || '');
  const [apiToken, setApiToken] = useState('');
  const [projects, setProjects] = useState(initialConfig?.projects?.join(', ') || '');
  const [isValidating, setIsValidating] = useState(false);
  const [validationResult, setValidationResult] = useState<{ valid: boolean; message: string } | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleValidate = async () => {
    setIsValidating(true);
    setValidationResult(null);
    try {
      const res = await api.validateCredentials({
        provider: 'asana',
        instance_url: 'https://app.asana.com/api/1.0',
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
      const projArray = projects.split(',').map((s) => s.trim()).filter(Boolean);

      await api.saveAsanaConnector({
        id,
        workspace: workspace || undefined,
        api_token: apiToken || undefined,
        projects: projArray.length > 0 ? projArray : undefined,
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
            <div className="w-6 h-6 rounded bg-rose-50 dark:bg-rose-500/20 text-rose-600 dark:text-rose-400 flex items-center justify-center font-bold text-xs border border-rose-200 dark:border-rose-500/30">
              AS
            </div>
            <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">Configure Asana Connector</h3>
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
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-rose-500 font-mono"
            />
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Workspace GID (Optional)</label>
            <input
              type="text"
              value={workspace}
              onChange={(e) => setWorkspace(e.target.value)}
              placeholder="9182390123"
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-rose-500 font-mono"
            />
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">
              Personal Access Token (PAT) {initialConfig && <span className="text-[10px] text-emerald-600 dark:text-emerald-400 font-normal">(Saved — leave empty to keep unchanged)</span>}
            </label>
            <input
              type="password"
              value={apiToken}
              onChange={(e) => setApiToken(e.target.value)}
              placeholder={initialConfig ? '•••••••• (Token Saved)' : '1/120938...'}
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-rose-500 font-mono"
            />
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Project GIDs (Comma separated)</label>
            <input
              type="text"
              value={projects}
              onChange={(e) => setProjects(e.target.value)}
              placeholder="12093810293, 12093810294"
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-rose-500 font-mono"
            />
          </div>

          {validationResult && (
            <div className={`p-2.5 rounded text-xs flex items-center gap-2 ${validationResult.valid ? 'bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-500/20' : 'bg-rose-50 dark:bg-rose-500/10 text-rose-600 dark:text-rose-400 border border-rose-200 dark:border-rose-500/20'}`}>
              {validationResult.valid ? <Check className="w-4 h-4 shrink-0" /> : <AlertCircle className="w-4 h-4 shrink-0" />}
              <span>{validationResult.message}</span>
            </div>
          )}

          <div className="flex items-center justify-between pt-3 border-t border-slate-200 dark:border-zinc-800">
            <button
              type="button"
              onClick={handleValidate}
              disabled={isValidating || !apiToken}
              className="px-3 py-1.5 rounded border border-slate-200 dark:border-zinc-700 text-slate-700 dark:text-zinc-300 hover:bg-slate-50 dark:hover:bg-zinc-800 disabled:opacity-50 flex items-center gap-1.5 font-medium"
            >
              {isValidating && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
              Test Connection
            </button>

            <div className="flex gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-3 py-1.5 rounded text-slate-600 dark:text-zinc-400 hover:bg-slate-100 dark:hover:bg-zinc-800"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-1.5 rounded bg-rose-600 hover:bg-rose-700 text-white font-medium flex items-center gap-1.5 disabled:opacity-50 shadow-sm"
              >
                {isSubmitting && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
                Save Connector
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};
