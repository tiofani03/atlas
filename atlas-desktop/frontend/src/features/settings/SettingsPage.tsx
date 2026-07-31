import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../services/api';
import { Settings, HardDrive, FileText, ShieldAlert } from 'lucide-react';

export const SettingsPage: React.FC = () => {
  const { data: status } = useQuery({ queryKey: ['status'], queryFn: api.getStatus });

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
    </div>
  );
};
