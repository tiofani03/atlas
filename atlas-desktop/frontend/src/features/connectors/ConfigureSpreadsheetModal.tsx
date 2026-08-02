import React, { useState } from 'react';
import { X, Folder, Check, AlertCircle, Loader2 } from 'lucide-react';
import { api } from '../../services/api';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  initialConfig?: {
    id: string;
    path?: string;
    paths?: string[];
  };
}

export const ConfigureSpreadsheetModal: React.FC<ModalProps> = ({ isOpen, onClose, onSuccess, initialConfig }) => {
  const [id, setId] = useState(initialConfig?.id || 'spreadsheet-main');
  const [paths, setPaths] = useState(initialConfig?.paths?.join(', ') || initialConfig?.path || '');
  const [apiToken, setApiToken] = useState('');
  const [hasHeaderRow, setHasHeaderRow] = useState(true);
  const [maxRows, setMaxRows] = useState(10000);
  const [isValidating, setIsValidating] = useState(false);
  const [validationResult, setValidationResult] = useState<{ valid: boolean; message: string } | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleBrowse = async () => {
    try {
      const res = await api.selectFolder();
      if (res.success && res.path) {
        setPaths((prev) => (prev ? `${prev}, ${res.path}` : res.path || ''));
      }
    } catch (err: unknown) {
      console.error(err);
    }
  };

  const handleValidate = async () => {
    setIsValidating(true);
    setValidationResult(null);
    try {
      const res = await api.validateCredentials({
        provider: 'spreadsheet',
        instance_url: '',
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
      const pathArray = paths.split(',').map((s) => s.trim()).filter(Boolean);

      await api.saveSpreadsheetConnector({
        id,
        path: pathArray[0] || undefined,
        paths: pathArray.length > 0 ? pathArray : undefined,
        api_token: apiToken || undefined,
        has_header_row: hasHeaderRow,
        max_rows_per_sheet: maxRows,
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
            <div className="w-6 h-6 rounded bg-teal-50 dark:bg-teal-500/20 text-teal-600 dark:text-teal-400 flex items-center justify-center font-bold text-xs border border-teal-200 dark:border-teal-500/30">
              SS
            </div>
            <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">Configure Spreadsheet Connector</h3>
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
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-teal-500 font-mono"
            />
          </div>

          <div>
            <div className="flex justify-between items-center mb-1">
              <label className="text-slate-600 dark:text-zinc-400 font-medium">Spreadsheet File Paths or Google Sheet URLs</label>
              <button
                type="button"
                onClick={handleBrowse}
                className="text-teal-600 hover:text-teal-700 dark:text-teal-400 font-medium flex items-center gap-1"
              >
                <Folder className="w-3 h-3" /> Browse
              </button>
            </div>
            <textarea
              rows={3}
              value={paths}
              onChange={(e) => setPaths(e.target.value)}
              placeholder="/path/to/data.csv, https://docs.google.com/spreadsheets/d/YOUR_SHEET_ID/edit#gid=0"
              required
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-teal-500 font-mono"
            />
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Google OAuth Token / API Key (Optional for private sheets)</label>
            <input
              type="password"
              value={apiToken}
              onChange={(e) => setApiToken(e.target.value)}
              placeholder={initialConfig ? 'Leave blank to keep existing token' : 'ya29.a0... or Google API Key'}
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-teal-500 font-mono"
            />
          </div>

          <div className="flex items-center gap-2 pt-1">
            <input
              type="checkbox"
              id="headerRow"
              checked={hasHeaderRow}
              onChange={(e) => setHasHeaderRow(e.target.checked)}
              className="rounded text-teal-600 focus:ring-teal-500"
            />
            <label htmlFor="headerRow" className="text-slate-700 dark:text-zinc-300 font-medium cursor-pointer">
              First row contains column headers
            </label>
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Max Rows Limit</label>
            <input
              type="number"
              value={maxRows}
              onChange={(e) => setMaxRows(parseInt(e.target.value, 10) || 10000)}
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-teal-500 font-mono"
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
                className="px-4 py-1.5 rounded bg-teal-600 hover:bg-teal-700 text-white font-medium flex items-center gap-1.5 disabled:opacity-50 shadow-sm"
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
