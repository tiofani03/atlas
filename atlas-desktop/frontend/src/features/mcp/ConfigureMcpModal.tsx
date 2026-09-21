import React, { useState, useEffect } from 'react';
import {
  X,
  Check,
  AlertCircle,
  Loader2,
  Plus,
  Trash2,
  Server,
  Play,
  Key,
  Layers,
  Terminal,
  Shield,
  Sliders,
  Lock,
  Wrench,
} from 'lucide-react';
import { api } from '../../services/api';
import { McpServerInfo, McpTestResult } from '../../types';

export interface SuggestedEnvVar {
  key: string;
  label: string;
  placeholder: string;
  description?: string;
  required?: boolean;
}

export interface McpCatalogItem {
  id: string;
  name: string;
  subtitle: string;
  description: string;
  category: 'design' | 'dev' | 'pm' | 'data' | 'system';
  icon: React.ReactNode;
  iconBgClass: string;
  defaultCommand: string;
  defaultArgs: string[];
  defaultPrefix: string;
  suggestedEnv: SuggestedEnvVar[];
  tag?: { label: string; color: 'indigo' | 'rose' | 'amber' | 'blue' | 'emerald' | 'purple' | 'slate' };
}

interface ConfigureMcpModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  catalogItem?: McpCatalogItem | null;
  initialServer?: McpServerInfo | null;
}

interface EnvRow {
  key: string;
  value: string;
  isExistingKey?: boolean;
}

function parseArgsString(str: string): string[] {
  const regex = /[^\s"']+|"([^"]*)"|'([^']*)'/g;
  const matches: string[] = [];
  let match;
  while ((match = regex.exec(str)) !== null) {
    matches.push(match[1] ?? match[2] ?? match[0]);
  }
  return matches;
}

export const ConfigureMcpModal: React.FC<ConfigureMcpModalProps> = ({
  isOpen,
  onClose,
  onSuccess,
  catalogItem,
  initialServer,
}) => {
  const isEditMode = !!initialServer;
  const isPresetLocked = !!catalogItem;

  const [name, setName] = useState('');
  const [command, setCommand] = useState('');
  const [argsInput, setArgsInput] = useState('');
  const [prefix, setPrefix] = useState('');
  const [enabled, setEnabled] = useState(true);

  // Suggested env values for catalog item
  const [suggestedEnvValues, setSuggestedEnvValues] = useState<Record<string, string>>({});

  // Additional custom env rows
  const [customEnvRows, setCustomEnvRows] = useState<EnvRow[]>([]);

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<McpTestResult | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Populate state whenever modal opens or inputs change
  useEffect(() => {
    if (!isOpen) return;

    setErrorMessage(null);
    setTestResult(null);

    if (catalogItem) {
      // Preset provider mode (e.g. Figma, GitHub, ClickUp)
      setName(catalogItem.id);
      setPrefix(initialServer?.prefix ?? catalogItem.defaultPrefix);
      setCommand(initialServer?.command || catalogItem.defaultCommand);
      setArgsInput(
        initialServer?.args ? initialServer.args.join(' ') : catalogItem.defaultArgs.join(' ')
      );
      setEnabled(initialServer?.enabled ?? true);

      // Initialize suggested env map
      const initialSuggested: Record<string, string> = {};
      const knownKeys = new Set(catalogItem.suggestedEnv.map((e) => e.key));
      for (const envDef of catalogItem.suggestedEnv) {
        initialSuggested[envDef.key] = '';
      }
      setSuggestedEnvValues(initialSuggested);

      // Any other custom env keys that might already exist
      if (initialServer) {
        const extraRows: EnvRow[] = initialServer.env_keys
          .filter((k) => !knownKeys.has(k))
          .map((k) => ({
            key: k,
            value: '',
            isExistingKey: true,
          }));
        setCustomEnvRows(extraRows);
      } else {
        setCustomEnvRows([]);
      }
    } else if (initialServer) {
      // Custom server edit mode
      setName(initialServer.name);
      setCommand(initialServer.command);
      setArgsInput(initialServer.args.join(' '));
      setPrefix(initialServer.prefix || '');
      setEnabled(initialServer.enabled);

      const existingRows: EnvRow[] = initialServer.env_keys.map((k) => ({
        key: k,
        value: '',
        isExistingKey: true,
      }));
      setCustomEnvRows(existingRows.length > 0 ? existingRows : [{ key: '', value: '' }]);
      setSuggestedEnvValues({});
    } else {
      // Brand new custom server
      setName('');
      setCommand('');
      setArgsInput('');
      setPrefix('');
      setEnabled(true);
      setCustomEnvRows([{ key: '', value: '' }]);
      setSuggestedEnvValues({});
    }
  }, [isOpen, catalogItem, initialServer]);

  if (!isOpen) return null;

  const buildEnvMap = (): Record<string, string> => {
    const env: Record<string, string> = {};

    // 1. Suggested env fields
    if (catalogItem) {
      for (const [k, v] of Object.entries(suggestedEnvValues)) {
        if (v && v.trim()) {
          env[k] = v.trim();
        }
      }
    }

    // 2. Custom env rows
    for (const row of customEnvRows) {
      const k = row.key.trim();
      const v = row.value.trim();
      if (k && v) {
        env[k] = v;
      }
    }

    return env;
  };

  const handleTest = async () => {
    setIsTesting(true);
    setTestResult(null);
    setErrorMessage(null);

    try {
      const serverName = name.trim() || catalogItem?.id || 'test_server';
      const parsedArgs = parseArgsString(argsInput);
      const envMap = buildEnvMap();

      const result = await api.testMcpServer(serverName, {
        command: command.trim(),
        args: parsedArgs,
        env: Object.keys(envMap).length > 0 ? envMap : undefined,
        prefix: prefix.trim() || undefined,
        enabled,
      });

      setTestResult(result);
    } catch (err: unknown) {
      setTestResult({
        success: false,
        error: (err as Error).message || 'Connection test failed',
      });
    } finally {
      setIsTesting(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    setErrorMessage(null);

    try {
      const serverName = name.trim() || catalogItem?.id;
      if (!serverName) {
        throw new Error('Server name is required');
      }
      if (!command.trim()) {
        throw new Error('Command executable is required (e.g. npx, python3, uvx)');
      }

      const parsedArgs = parseArgsString(argsInput);
      const envMap = buildEnvMap();

      await api.saveMcpServer({
        name: serverName,
        command: command.trim(),
        args: parsedArgs,
        env: envMap,
        prefix: prefix.trim() || undefined,
        enabled,
      });

      onSuccess();
      onClose();
    } catch (err: unknown) {
      setErrorMessage((err as Error).message || 'Failed to save MCP server');
    } finally {
      setIsSubmitting(false);
    }
  };

  const addCustomEnvRow = () => {
    setCustomEnvRows([...customEnvRows, { key: '', value: '' }]);
  };

  const removeCustomEnvRow = (index: number) => {
    setCustomEnvRows(customEnvRows.filter((_, i) => i !== index));
  };

  const updateCustomEnvRow = (index: number, field: 'key' | 'value', val: string) => {
    const updated = [...customEnvRows];
    updated[index][field] = val;
    setCustomEnvRows(updated);
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-900/50 dark:bg-black/80 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-2xl w-full max-w-xl p-6 space-y-5 shadow-2xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-slate-200 dark:border-zinc-800 pb-3 shrink-0">
          <div className="flex items-center gap-3">
            <div
              className={`w-9 h-9 rounded-xl flex items-center justify-center border shadow-xs ${
                catalogItem
                  ? catalogItem.iconBgClass
                  : 'bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-500/30'
              }`}
            >
              {catalogItem ? catalogItem.icon : <Server className="w-5 h-5" />}
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">
                  {catalogItem
                    ? `${isEditMode ? 'Edit' : 'Configure'} ${catalogItem.name}`
                    : isEditMode
                    ? `Edit ${initialServer?.name} MCP`
                    : 'Add Custom MCP Server'}
                </h3>
                {isPresetLocked && (
                  <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-full bg-slate-100 dark:bg-zinc-800 text-slate-600 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700">
                    <Lock className="w-2.5 h-2.5" />
                    <span>Locked Provider</span>
                  </span>
                )}
              </div>
              <p className="text-xs text-slate-500 dark:text-zinc-400">
                {catalogItem
                  ? catalogItem.description
                  : 'Connect any local stdio or external process MCP server into the Atlas Hub'}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-600 dark:text-zinc-400 dark:hover:text-zinc-200"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Scrollable Form Body */}
        <form onSubmit={handleSubmit} className="space-y-4 overflow-y-auto flex-1 pr-1 text-xs">
          {/* Server Identity: Name and Tool Prefix */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div>
              <label className="block text-slate-700 dark:text-zinc-300 font-semibold mb-1 flex items-center justify-between">
                <span>Server ID / Name</span>
                {isPresetLocked && (
                  <span className="text-[10px] text-slate-400 font-normal font-mono">Fixed</span>
                )}
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                disabled={isPresetLocked || isEditMode}
                required
                placeholder="e.g. figma, github, custom-tool"
                className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono disabled:opacity-70 disabled:bg-slate-100 dark:disabled:bg-zinc-800/40"
              />
            </div>

            <div>
              <label className="block text-slate-700 dark:text-zinc-300 font-semibold mb-1 flex items-center justify-between">
                <span>Tool Namespace Prefix</span>
                <span className="text-[10px] text-slate-400 font-normal">Optional</span>
              </label>
              <input
                type="text"
                value={prefix}
                onChange={(e) => setPrefix(e.target.value)}
                placeholder={catalogItem ? catalogItem.defaultPrefix : 'e.g. figma, gh'}
                className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
              />
              <p className="text-[10px] text-slate-400 dark:text-zinc-500 mt-1">
                Tools will be exposed as <span className="font-mono text-indigo-500">{prefix || name || 'prefix'}__tool</span>
              </p>
            </div>
          </div>

          {/* Command Executable & Arguments */}
          <div className="space-y-3 p-3.5 bg-slate-50/70 dark:bg-zinc-950/60 rounded-xl border border-slate-200 dark:border-zinc-800/80">
            <div className="flex items-center gap-1.5 text-slate-700 dark:text-zinc-300 font-semibold">
              <Terminal className="w-3.5 h-3.5 text-indigo-500" />
              <span>Process Execution</span>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
              <div className="sm:col-span-1">
                <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">
                  Executable
                </label>
                <input
                  type="text"
                  value={command}
                  onChange={(e) => setCommand(e.target.value)}
                  required
                  placeholder="npx / python3 / uvx"
                  className="w-full bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-lg px-2.5 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
                />
              </div>

              <div className="sm:col-span-2">
                <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">
                  Arguments (Space-separated)
                </label>
                <input
                  type="text"
                  value={argsInput}
                  onChange={(e) => setArgsInput(e.target.value)}
                  placeholder='-y @modelcontextprotocol/server-figma'
                  className="w-full bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-lg px-2.5 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
                />
              </div>
            </div>
          </div>

          {/* Provider Specific Suggested Environment Variables */}
          {catalogItem && catalogItem.suggestedEnv.length > 0 && (
            <div className="space-y-3 p-3.5 bg-slate-50/70 dark:bg-zinc-950/60 rounded-xl border border-slate-200 dark:border-zinc-800/80">
              <div className="flex items-center gap-1.5 text-slate-700 dark:text-zinc-300 font-semibold">
                <Key className="w-3.5 h-3.5 text-indigo-500" />
                <span>Required Credentials & API Tokens</span>
              </div>

              <div className="space-y-2.5">
                {catalogItem.suggestedEnv.map((envDef) => {
                  const isSaved = initialServer?.env_keys.includes(envDef.key);
                  return (
                    <div key={envDef.key} className="space-y-1">
                      <div className="flex items-center justify-between">
                        <label className="text-slate-600 dark:text-zinc-400 font-medium flex items-center gap-1.5">
                          <span>{envDef.label}</span>
                          <span className="font-mono text-[10px] text-slate-400">({envDef.key})</span>
                        </label>
                        {isSaved && (
                          <span className="text-[10px] text-emerald-600 dark:text-emerald-400 font-medium flex items-center gap-1">
                            <Check className="w-3 h-3" />
                            <span>Saved (leave blank to keep)</span>
                          </span>
                        )}
                      </div>
                      <input
                        type="password"
                        value={suggestedEnvValues[envDef.key] || ''}
                        onChange={(e) =>
                          setSuggestedEnvValues({
                            ...suggestedEnvValues,
                            [envDef.key]: e.target.value,
                          })
                        }
                        placeholder={isSaved ? '••••••••••••••••' : envDef.placeholder}
                        className="w-full bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-lg px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500 font-mono"
                      />
                      {envDef.description && (
                        <p className="text-[10px] text-slate-500 dark:text-zinc-400">
                          {envDef.description}
                        </p>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Custom Environment Variables */}
          <div className="space-y-2.5 p-3.5 bg-slate-50/70 dark:bg-zinc-950/60 rounded-xl border border-slate-200 dark:border-zinc-800/80">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5 text-slate-700 dark:text-zinc-300 font-semibold">
                <Sliders className="w-3.5 h-3.5 text-indigo-500" />
                <span>
                  {catalogItem ? 'Additional Environment Variables' : 'Environment Variables'}
                </span>
              </div>
              <button
                type="button"
                onClick={addCustomEnvRow}
                className="text-[11px] text-indigo-600 dark:text-indigo-400 hover:text-indigo-700 dark:hover:text-indigo-300 font-medium flex items-center gap-1"
              >
                <Plus className="w-3 h-3" />
                <span>Add Variable</span>
              </button>
            </div>

            {customEnvRows.length === 0 ? (
              <p className="text-[11px] text-slate-400 dark:text-zinc-500 italic">
                No custom environment variables defined.
              </p>
            ) : (
              <div className="space-y-2">
                {customEnvRows.map((row, idx) => (
                  <div key={idx} className="flex items-center gap-2">
                    <input
                      type="text"
                      value={row.key}
                      onChange={(e) => updateCustomEnvRow(idx, 'key', e.target.value)}
                      placeholder="KEY_NAME"
                      className="flex-1 bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-lg px-2.5 py-1.5 text-slate-900 dark:text-zinc-100 font-mono text-[11px] focus:outline-none focus:border-indigo-500"
                    />
                    <input
                      type="password"
                      value={row.value}
                      onChange={(e) => updateCustomEnvRow(idx, 'value', e.target.value)}
                      placeholder={row.isExistingKey ? '•••••••• (Saved)' : 'Value'}
                      className="flex-1 bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-lg px-2.5 py-1.5 text-slate-900 dark:text-zinc-100 font-mono text-[11px] focus:outline-none focus:border-indigo-500"
                    />
                    <button
                      type="button"
                      onClick={() => removeCustomEnvRow(idx)}
                      className="p-1.5 text-slate-400 hover:text-rose-500 rounded-lg transition"
                      title="Remove"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Enabled Toggle */}
          <div className="flex items-center gap-2 pt-1">
            <input
              type="checkbox"
              id="mcp-enabled-toggle"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="w-4 h-4 rounded text-indigo-600 focus:ring-indigo-500 border-slate-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 cursor-pointer"
            />
            <label
              htmlFor="mcp-enabled-toggle"
              className="text-xs text-slate-700 dark:text-zinc-300 font-medium cursor-pointer"
            >
              Enable this server in Atlas MCP Hub
            </label>
          </div>

          {/* Test Connection Live Banner / Output */}
          {testResult && (
            <div
              className={`p-3 rounded-xl border flex items-start gap-2.5 ${
                testResult.success
                  ? 'bg-emerald-50 dark:bg-emerald-950/40 text-emerald-800 dark:text-emerald-300 border-emerald-200 dark:border-emerald-800/60'
                  : 'bg-rose-50 dark:bg-rose-950/40 text-rose-800 dark:text-rose-300 border-rose-200 dark:border-rose-800/60'
              }`}
            >
              {testResult.success ? (
                <Check className="w-4 h-4 text-emerald-600 dark:text-emerald-400 shrink-0 mt-0.5" />
              ) : (
                <AlertCircle className="w-4 h-4 text-rose-600 dark:text-rose-400 shrink-0 mt-0.5" />
              )}
              <div className="space-y-1 flex-1 min-w-0">
                <p className="font-semibold text-xs">
                  {testResult.success
                    ? 'Connection Successful!'
                    : 'Connection Test Failed'}
                </p>
                <p className="text-[11px] leading-relaxed">
                  {testResult.success
                    ? testResult.message ||
                      `Successfully connected and discovered ${testResult.tools?.length || 0} tools.`
                    : testResult.error}
                </p>
                {testResult.tools && testResult.tools.length > 0 && (
                  <div className="flex flex-wrap gap-1 pt-1">
                    {testResult.tools.slice(0, 6).map((t) => (
                      <span
                        key={t.name}
                        className="font-mono text-[10px] px-1.5 py-0.5 rounded bg-white/60 dark:bg-black/30 border border-emerald-200 dark:border-emerald-800/60"
                      >
                        {t.name}
                      </span>
                    ))}
                    {testResult.tools.length > 6 && (
                      <span className="font-mono text-[10px] text-slate-500 dark:text-zinc-400 px-1 py-0.5">
                        +{testResult.tools.length - 6} more
                      </span>
                    )}
                  </div>
                )}
              </div>
            </div>
          )}

          {errorMessage && (
            <div className="p-3 rounded-xl bg-rose-50 dark:bg-rose-950/40 text-rose-800 dark:text-rose-300 border border-rose-200 dark:border-rose-800/60 flex items-center gap-2">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>{errorMessage}</span>
            </div>
          )}

          {/* Footer Actions */}
          <div className="flex items-center justify-between pt-3 border-t border-slate-200 dark:border-zinc-800 shrink-0">
            <button
              type="button"
              onClick={handleTest}
              disabled={isTesting || !command.trim()}
              className="px-3.5 py-1.5 rounded-xl border border-slate-200 dark:border-zinc-700 text-slate-700 dark:text-zinc-300 hover:bg-slate-50 dark:hover:bg-zinc-800 disabled:opacity-50 flex items-center gap-1.5 font-medium transition shadow-2xs"
            >
              {isTesting ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin text-indigo-500" />
              ) : (
                <Play className="w-3.5 h-3.5 text-indigo-500" />
              )}
              <span>{isTesting ? 'Testing...' : 'Test Connection'}</span>
            </button>

            <div className="flex gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-3.5 py-1.5 rounded-xl text-slate-600 dark:text-zinc-400 hover:bg-slate-100 dark:hover:bg-zinc-800 font-medium transition"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting || !command.trim()}
                className="px-4 py-1.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-semibold flex items-center gap-1.5 disabled:opacity-50 shadow-xs active:scale-95 transition"
              >
                {isSubmitting && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
                <span>Save MCP Server</span>
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};
