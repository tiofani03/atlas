import React, { useEffect, useState } from 'react';
import { api } from '../../services/api';
import type { ConfigFieldSchema, ConnectorInfo, ConnectorManifest } from '../../types';
import { CheckCircle2, FolderOpen, Loader2, ShieldCheck, X, AlertCircle } from 'lucide-react';

interface DynamicConnectorModalProps {
  isOpen: boolean;
  manifest: ConnectorManifest | null;
  initialConnector?: ConnectorInfo;
  onClose: () => void;
  onSuccess: () => void;
}

export const DynamicConnectorModal: React.FC<DynamicConnectorModalProps> = ({
  isOpen,
  manifest,
  initialConnector,
  onClose,
  onSuccess,
}) => {
  const [connectorId, setConnectorId] = useState('');
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [validationResult, setValidationResult] = useState<{ valid: boolean; message: string } | null>(null);

  useEffect(() => {
    if (!isOpen || !manifest) return;
    setError(null);
    setValidationResult(null);
    setConnectorId(initialConnector?.id || `${manifest.provider}-main`);

    const init: Record<string, unknown> = {};
    for (const field of manifest.config_fields) {
      const existing = initialConnector
        ? (initialConnector as unknown as Record<string, unknown>)[field.key]
        : undefined;

      if (field.field_type === 'list') {
        if (Array.isArray(existing)) {
          init[field.key] = existing.join(', ');
        } else {
          init[field.key] = '';
        }
      } else if (field.field_type === 'path') {
        if (Array.isArray(existing) && existing.length > 0) {
          init[field.key] = existing.join(', ');
        } else if (typeof existing === 'string') {
          init[field.key] = existing;
        } else if (field.key === 'paths' && initialConnector?.path) {
          init[field.key] = initialConnector.path;
        } else {
          init[field.key] = '';
        }
      } else if (field.field_type === 'boolean') {
        init[field.key] = Boolean(existing);
      } else {
        init[field.key] = typeof existing === 'string' ? existing : '';
      }
    }
    setValues(init);
  }, [isOpen, manifest, initialConnector]);

  if (!isOpen) return null;

  if (!manifest) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs">
        <div className="bg-white dark:bg-zinc-900 rounded-2xl max-w-md w-full border border-slate-200 dark:border-zinc-800 shadow-2xl p-6 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">Connector Schema Unavailable</h3>
            <button type="button" onClick={onClose} className="p-1 rounded-lg text-slate-400 hover:text-slate-600">
              <X className="w-4 h-4" />
            </button>
          </div>
          <p className="text-xs text-slate-600 dark:text-zinc-400">
            Schema konfigurasi untuk connector ini belum termuat dari backend (`/api/connectors/schemas`). Pastikan server backend terbaru sedang berjalan lalu muat ulang halaman.
          </p>
        </div>
      </div>
    );
  }

  const buildPayload = () => {
    const payload: Record<string, unknown> = {
      id: connectorId.trim() || `${manifest.provider}-main`,
    };
    for (const field of manifest.config_fields) {
      const raw = values[field.key];
      if (field.field_type === 'list' || field.key === 'paths') {
        const str = typeof raw === 'string' ? raw : '';
        const arr = str
          .split(/[,\n]/)
          .map((s) => s.trim())
          .filter(Boolean);
        if (arr.length > 0) {
          payload[field.key] = arr;
        }
      } else if (field.field_type === 'boolean') {
        payload[field.key] = Boolean(raw);
      } else {
        const str = typeof raw === 'string' ? raw.trim() : '';
        if (str.length > 0) {
          payload[field.key] = str;
        }
      }
    }
    return payload;
  };

  const handleBrowseFolder = async (fieldKey: string) => {
    try {
      const res = await api.selectFolder();
      if (res.success && res.path) {
        const current = typeof values[fieldKey] === 'string' ? (values[fieldKey] as string).trim() : '';
        const next = current ? `${current}, ${res.path}` : res.path;
        setValues((prev) => ({ ...prev, [fieldKey]: next }));
      }
    } catch {
      // Ignore folder picker cancellation
    }
  };

  const handleValidate = async () => {
    setValidating(true);
    setValidationResult(null);
    setError(null);
    try {
      const payload = buildPayload();
      const res = await api.validateCredentials({
        provider: manifest.provider,
        ...(payload as Record<string, unknown>),
      });
      setValidationResult(res);
    } catch (err) {
      setValidationResult({
        valid: false,
        message: err instanceof Error ? err.message : 'Validation failed',
      });
    } finally {
      setValidating(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const payload = buildPayload();
      await api.saveConnectorGeneric(manifest.provider, payload);
      onSuccess();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save connector');
    } finally {
      setSaving(false);
    }
  };

  const renderField = (field: ConfigFieldSchema) => {
    const val = values[field.key];
    if (field.field_type === 'boolean') {
      return (
        <label className="flex items-center gap-2.5 cursor-pointer select-none">
          <input
            type="checkbox"
            checked={Boolean(val)}
            onChange={(e) => setValues((prev) => ({ ...prev, [field.key]: e.target.checked }))}
            className="w-4 h-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
          />
          <span className="text-xs font-medium text-slate-700 dark:text-zinc-300">
            {field.help_text || field.label}
          </span>
        </label>
      );
    }

    return (
      <div className="flex gap-2">
        <input
          type={field.field_type === 'password' ? 'password' : field.field_type === 'url' ? 'url' : 'text'}
          value={typeof val === 'string' ? val : ''}
          onChange={(e) => setValues((prev) => ({ ...prev, [field.key]: e.target.value }))}
          placeholder={field.placeholder}
          required={field.required && !(field.field_type === 'password' && Boolean(initialConnector))}
          className="flex-1 px-3 py-2 text-xs rounded-xl bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 text-slate-900 dark:text-zinc-100 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500/40"
        />
        {field.field_type === 'path' && (
          <button
            type="button"
            onClick={() => handleBrowseFolder(field.key)}
            className="px-3 py-2 rounded-xl text-xs font-medium bg-slate-100 hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-200 border border-slate-200 dark:border-zinc-700 flex items-center gap-1.5 transition"
          >
            <FolderOpen className="w-3.5 h-3.5" />
            Browse
          </button>
        )}
      </div>
    );
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs">
      <div className="bg-white dark:bg-zinc-900 rounded-2xl max-w-lg w-full border border-slate-200 dark:border-zinc-800 shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-100 dark:border-zinc-800">
          <div>
            <h3 className="text-base font-bold text-slate-900 dark:text-zinc-100">
              Configure {manifest.name}
            </h3>
            <p className="text-xs text-slate-500 dark:text-zinc-400 mt-0.5">{manifest.description}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close modal"
            className="p-1.5 rounded-lg text-slate-400 hover:text-slate-600 dark:hover:text-zinc-200 hover:bg-slate-100 dark:hover:bg-zinc-800 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-4 max-h-[75vh] overflow-y-auto">
          <div>
            <label className="block text-xs font-semibold text-slate-700 dark:text-zinc-300 mb-1.5">
              Connector ID <span className="text-rose-500">*</span>
            </label>
            <input
              type="text"
              value={connectorId}
              onChange={(e) => setConnectorId(e.target.value)}
              disabled={Boolean(initialConnector)}
              required
              className="w-full px-3 py-2 text-xs font-mono rounded-xl bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 text-slate-900 dark:text-zinc-100 disabled:opacity-60 focus:outline-none focus:ring-2 focus:ring-indigo-500/40"
            />
          </div>

          {manifest.config_fields.map((field) => (
            <div key={field.key}>
              <label className="block text-xs font-semibold text-slate-700 dark:text-zinc-300 mb-1.5">
                {field.label}
                {field.required && <span className="text-rose-500 ml-0.5">*</span>}
              </label>
              {renderField(field)}
              {field.help_text && field.field_type !== 'boolean' && (
                <p className="text-[11px] text-slate-400 dark:text-zinc-500 mt-1">{field.help_text}</p>
              )}
            </div>
          ))}

          {error && (
            <div className="p-3 rounded-xl bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-800/50 text-xs text-rose-700 dark:text-rose-300 flex items-center gap-2">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {validationResult && (
            <div
              className={`p-3 rounded-xl border text-xs flex items-center gap-2 ${
                validationResult.valid
                  ? 'bg-emerald-50 dark:bg-emerald-950/40 border-emerald-200 dark:border-emerald-800/50 text-emerald-700 dark:text-emerald-300'
                  : 'bg-amber-50 dark:bg-amber-950/40 border-amber-200 dark:border-amber-800/50 text-amber-700 dark:text-amber-300'
              }`}
            >
              <CheckCircle2 className="w-4 h-4 shrink-0" />
              <span>{validationResult.message}</span>
            </div>
          )}

          <div className="flex items-center justify-between pt-3 border-t border-slate-100 dark:border-zinc-800">
            <button
              type="button"
              onClick={handleValidate}
              disabled={validating || saving}
              className="px-3.5 py-2 rounded-xl text-xs font-semibold bg-slate-100 hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-200 border border-slate-200 dark:border-zinc-700 flex items-center gap-1.5 transition disabled:opacity-50"
            >
              {validating ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <ShieldCheck className="w-3.5 h-3.5" />}
              Test Connection
            </button>

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 rounded-xl text-xs font-semibold text-slate-600 dark:text-zinc-400 hover:bg-slate-100 dark:hover:bg-zinc-800 transition"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={saving}
                className="px-4 py-2 rounded-xl text-xs font-bold bg-indigo-600 hover:bg-indigo-500 text-white shadow-md shadow-indigo-500/20 flex items-center gap-1.5 transition disabled:opacity-50"
              >
                {saving && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
                {saving ? 'Saving...' : 'Save Connector'}
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};
