import React, { useState } from 'react';
import { X, Check, AlertCircle, Loader2 } from 'lucide-react';
import { api } from '../../services/api';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export const ConfigureJiraModal: React.FC<ModalProps> = ({ isOpen, onClose, onSuccess }) => {
  const [id, setId] = useState('jira-main');
  const [instanceUrl, setInstanceUrl] = useState('https://company.atlassian.net');
  const [email, setEmail] = useState('');
  const [apiToken, setApiToken] = useState('');
  const [projects, setProjects] = useState('PAY,DEV');
  const [isValidating, setIsValidating] = useState(false);
  const [validationResult, setValidationResult] = useState<{ valid: boolean; message: string } | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleValidate = async () => {
    setIsValidating(true);
    setValidationResult(null);
    try {
      const res = await api.validateCredentials({
        provider: 'jira',
        instance_url: instanceUrl,
        email,
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
      const projArray = projects.split(',').map((p) => p.trim()).filter(Boolean);
      await api.saveJiraConnector({
        id,
        instance_url: instanceUrl,
        email,
        api_token: apiToken,
        projects: projArray,
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
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl w-full max-w-md p-6 space-y-4 shadow-2xl">
        <div className="flex items-center justify-between border-b border-zinc-800 pb-3">
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 rounded bg-blue-500/20 text-blue-400 flex items-center justify-center font-bold text-xs">
              J
            </div>
            <h3 className="text-sm font-bold text-zinc-100">Configure Jira Connector</h3>
          </div>
          <button onClick={onClose} className="text-zinc-400 hover:text-zinc-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-3 text-xs">
          <div>
            <label className="block text-zinc-400 font-medium mb-1">Connector ID</label>
            <input
              type="text"
              value={id}
              onChange={(e) => setId(e.target.value)}
              required
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
            />
          </div>

          <div>
            <label className="block text-zinc-400 font-medium mb-1">Base URL</label>
            <input
              type="url"
              value={instanceUrl}
              onChange={(e) => setInstanceUrl(e.target.value)}
              placeholder="https://company.atlassian.net"
              required
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-100 focus:outline-none focus:border-indigo-500"
            />
          </div>

          <div>
            <label className="block text-zinc-400 font-medium mb-1">Email</label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="user@company.com"
              required
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-100 focus:outline-none focus:border-indigo-500"
            />
          </div>

          <div>
            <label className="block text-zinc-400 font-medium mb-1">API Token</label>
            <input
              type="password"
              value={apiToken}
              onChange={(e) => setApiToken(e.target.value)}
              placeholder="Atlassian API Token"
              required
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-100 focus:outline-none focus:border-indigo-500"
            />
          </div>

          <div>
            <label className="block text-zinc-400 font-medium mb-1">Projects (comma-separated)</label>
            <input
              type="text"
              value={projects}
              onChange={(e) => setProjects(e.target.value)}
              placeholder="PAY, DEV, ARCH"
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
            />
          </div>

          {validationResult && (
            <div
              className={`p-2.5 rounded text-xs flex items-center gap-2 ${
                validationResult.valid
                  ? 'bg-emerald-950/50 border border-emerald-800/50 text-emerald-400'
                  : 'bg-rose-950/50 border border-rose-800/50 text-rose-400'
              }`}
            >
              {validationResult.valid ? (
                <Check className="w-4 h-4 shrink-0 text-emerald-400" />
              ) : (
                <AlertCircle className="w-4 h-4 shrink-0 text-rose-400" />
              )}
              <span>{validationResult.message}</span>
            </div>
          )}

          <div className="flex items-center justify-between border-t border-zinc-800 pt-3 mt-4">
            <button
              type="button"
              onClick={handleValidate}
              disabled={isValidating || !email || !apiToken}
              className="px-3 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 font-medium transition disabled:opacity-50 flex items-center gap-1.5"
            >
              {isValidating && <Loader2 className="w-3 h-3 animate-spin" />}
              <span>Validate</span>
            </button>

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-3 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 transition"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-1.5 rounded bg-indigo-600 hover:bg-indigo-500 text-white font-medium transition disabled:opacity-50"
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
