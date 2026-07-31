import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../services/api';
import { Settings, HardDrive, ShieldAlert, Sparkles, Trash2, AlertTriangle, RefreshCw } from 'lucide-react';

export const SettingsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const [showConfirmModal, setShowConfirmModal] = useState(false);
  const [showSuccessToast, setShowSuccessToast] = useState(false);

  const { data: status } = useQuery({ queryKey: ['status'], queryFn: api.getStatus });

  const clearDataMutation = useMutation({
    mutationFn: () => api.clearData(),
    onSuccess: () => {
      setShowConfirmModal(false);
      setShowSuccessToast(true);
      queryClient.invalidateQueries({ queryKey: ['status'] });
      queryClient.invalidateQueries({ queryKey: ['syncStatus'] });
      queryClient.invalidateQueries({ queryKey: ['knowledge'] });
      setTimeout(() => setShowSuccessToast(false), 4000);
    },
  });

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      <div>
        <h2 className="text-xl font-bold text-slate-900 dark:text-zinc-100 tracking-tight">Settings</h2>
        <p className="text-xs text-slate-500 dark:text-zinc-400 mt-1">
          Local engine paths, configuration parameters, and storage state.
        </p>
      </div>

      <div className="glass-card p-6 rounded-xl border border-slate-200 dark:border-zinc-800 space-y-4 max-w-2xl shadow-xs">
        <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-200 border-b border-slate-200 dark:border-zinc-800 pb-3 flex items-center gap-2">
          <HardDrive className="w-4 h-4 text-indigo-600 dark:text-indigo-400" />
          <span>Local Engine Configuration</span>
        </h3>

        <div className="space-y-3 text-xs">
          <div>
            <label className="block text-slate-500 dark:text-zinc-500 font-medium mb-1">Config File Path (~/.config/atlas/config.toml)</label>
            <input
              type="text"
              readOnly
              value={status?.config_path || ''}
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg px-3 py-2 text-slate-900 dark:text-zinc-300 font-mono select-all focus:outline-none"
            />
          </div>

          <div>
            <label className="block text-slate-500 dark:text-zinc-500 font-medium mb-1">SQLite Database Location (~/.local/share/atlas/atlas.db)</label>
            <input
              type="text"
              readOnly
              value={status?.db_path || ''}
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg px-3 py-2 text-slate-900 dark:text-zinc-300 font-mono select-all focus:outline-none"
            />
          </div>

          <div className="pt-2">
            <div className="p-3 rounded-lg bg-indigo-50 dark:bg-indigo-950/30 border border-indigo-200 dark:border-indigo-800/30 text-indigo-700 dark:text-indigo-300 text-xs space-y-1">
              <p className="font-semibold flex items-center gap-1.5">
                <ShieldAlert className="w-4 h-4 text-indigo-600 dark:text-indigo-400" />
                <span>Local-First Execution Model</span>
              </p>
              <p className="text-[11px] text-indigo-600/80 dark:text-indigo-300/80">
                Atlas Desktop operates strictly as a localhost companion host. All settings modified via UI write directly to your local TOML file.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Danger Zone: Reset Database */}
      <div className="p-6 rounded-xl border border-rose-200 dark:border-rose-900/40 bg-rose-50/40 dark:bg-rose-950/20 space-y-4 max-w-2xl shadow-xs">
        <h3 className="text-sm font-bold text-rose-700 dark:text-rose-400 border-b border-rose-200 dark:border-rose-900/40 pb-3 flex items-center gap-2">
          <AlertTriangle className="w-4 h-4 text-rose-500" />
          <span>Danger Zone — Database Management</span>
        </h3>

        <div className="flex items-center justify-between gap-4">
          <div>
            <h4 className="text-xs font-bold text-slate-900 dark:text-zinc-100 flex items-center gap-1.5">
              <Trash2 className="w-3.5 h-3.5 text-rose-500" />
              <span>Clear All Context Data</span>
            </h4>
            <p className="text-[11px] text-slate-500 dark:text-zinc-400 mt-0.5">
              Permanently purge all synchronized artifacts, graph edges, and search indices from your local SQLite database.
            </p>
          </div>

          <button
            onClick={() => setShowConfirmModal(true)}
            className="flex items-center gap-1.5 text-xs px-4 py-2 rounded-lg bg-rose-600 hover:bg-rose-500 active:bg-rose-700 text-white font-bold transition shadow-xs hover:scale-105 active:scale-95 shrink-0"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Clear Data</span>
          </button>
        </div>
      </div>

      {/* Confirmation Modal */}
      {showConfirmModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 animate-in fade-in duration-150">
          <div className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-xl p-6 max-w-md w-full shadow-2xl space-y-4">
            <div className="flex items-center gap-3 text-rose-600 dark:text-rose-400">
              <div className="p-2.5 rounded-full bg-rose-500/10 border border-rose-500/20">
                <AlertTriangle className="w-6 h-6 text-rose-500" />
              </div>
              <div>
                <h3 className="text-base font-bold text-slate-900 dark:text-zinc-100">
                  Reset Context Data?
                </h3>
                <p className="text-xs text-slate-500 dark:text-zinc-400">
                  This action cannot be undone.
                </p>
              </div>
            </div>

            <p className="text-xs text-slate-600 dark:text-zinc-300 leading-relaxed bg-slate-50 dark:bg-zinc-950 p-3 rounded-lg border border-slate-200 dark:border-zinc-800">
              All <strong>KnowledgeArtifact</strong> items, relationship graphs, connector states, and FTS5 search indices in SQLite will be permanently purged.
            </p>

            <div className="flex items-center justify-end gap-2 pt-2">
              <button
                onClick={() => setShowConfirmModal(false)}
                disabled={clearDataMutation.isPending}
                className="px-4 py-2 text-xs font-semibold rounded-lg text-slate-700 dark:text-zinc-300 hover:bg-slate-100 dark:hover:bg-zinc-800 transition"
              >
                Cancel
              </button>
              <button
                onClick={() => clearDataMutation.mutate()}
                disabled={clearDataMutation.isPending}
                className="flex items-center gap-1.5 px-4 py-2 text-xs font-bold rounded-lg bg-rose-600 hover:bg-rose-500 active:bg-rose-700 text-white shadow-xs transition disabled:opacity-50"
              >
                {clearDataMutation.isPending ? (
                  <>
                    <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                    <span>Clearing Data...</span>
                  </>
                ) : (
                  <>
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>Yes, Clear All Data</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Success Toast Notification */}
      {showSuccessToast && (
        <div className="fixed bottom-5 right-5 z-50 flex items-center gap-2 bg-emerald-600 text-white px-4 py-3 rounded-xl shadow-xl text-xs font-semibold animate-in slide-in-from-bottom-5 duration-200">
          <Sparkles className="w-4 h-4 text-emerald-200" />
          <span>All engineering context data has been cleared successfully.</span>
        </div>
      )}
    </div>
  );
};
