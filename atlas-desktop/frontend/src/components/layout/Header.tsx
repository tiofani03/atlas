import React from 'react';
import { RefreshCw, Terminal, Circle, Sun, Moon } from 'lucide-react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../services/api';

interface HeaderProps {
  currentTab: string;
  isDarkMode: boolean;
  onToggleTheme: () => void;
}

export const Header: React.FC<HeaderProps> = ({ currentTab, isDarkMode, onToggleTheme }) => {
  const queryClient = useQueryClient();
  const { data: status } = useQuery({ queryKey: ['status'], queryFn: api.getStatus });
  const { data: syncStatus } = useQuery({
    queryKey: ['syncStatus'],
    queryFn: api.getSyncStatus,
    refetchInterval: (query) => (query.state.data?.is_running ? 1000 : 5000),
  });

  const triggerSyncMutation = useMutation({
    mutationFn: (id?: string) => api.triggerSync(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['syncStatus'] });
      queryClient.invalidateQueries({ queryKey: ['status'] });
      queryClient.invalidateQueries({ queryKey: ['connectors'] });
    },
  });

  return (
    <header className="h-14 border-b border-slate-200 dark:border-zinc-800 bg-white/90 dark:bg-zinc-950/80 backdrop-blur-md px-6 flex items-center justify-between sticky top-0 z-30 transition-colors">
      <div className="flex items-center gap-3">
        <h1 className="text-sm font-semibold text-slate-900 dark:text-zinc-100 capitalize tracking-wide">
          {currentTab}
        </h1>
        <span className="text-xs px-2 py-0.5 rounded bg-slate-100 dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 text-slate-600 dark:text-zinc-400 font-mono font-medium">
          atx v{status?.version || '0.1.0'}
        </span>
      </div>

      <div className="flex items-center gap-3">
        {/* Status Pill */}
        <div className="flex items-center gap-2 text-xs px-2.5 py-1 rounded-full bg-emerald-50 dark:bg-emerald-950/50 border border-emerald-200 dark:border-emerald-500/30 text-emerald-700 dark:text-emerald-400 font-medium">
          <Circle className="w-2 h-2 fill-emerald-500 text-emerald-500 animate-pulse" />
          <span>Localhost Engine Online</span>
        </div>

        {/* CLI Companion Tag */}
        <div className="hidden sm:flex items-center gap-1.5 text-xs text-slate-500 dark:text-zinc-400">
          <Terminal className="w-3.5 h-3.5" />
          <span>CLI-First Engine</span>
        </div>

        {/* Theme Toggle Button */}
        <button
          onClick={onToggleTheme}
          title={isDarkMode ? 'Switch to Light Mode' : 'Switch to Dark Mode'}
          className="p-1.5 rounded-lg border border-slate-200 dark:border-zinc-800 bg-slate-100 hover:bg-slate-200 dark:bg-zinc-900 dark:hover:bg-zinc-800 text-slate-700 dark:text-zinc-300 transition flex items-center justify-center shadow-xs"
        >
          {isDarkMode ? <Sun className="w-4 h-4 text-indigo-400" /> : <Moon className="w-4 h-4 text-slate-700" />}
        </button>

        {/* Global Sync Trigger Button */}
        <button
          onClick={() => triggerSyncMutation.mutate(undefined)}
          disabled={syncStatus?.is_running || triggerSyncMutation.isPending}
          className="flex items-center gap-1.5 text-xs px-3.5 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 text-white font-bold transition disabled:opacity-50 disabled:cursor-not-allowed shadow-xs"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${syncStatus?.is_running ? 'animate-spin' : ''}`} />
          <span>{syncStatus?.is_running ? 'Syncing...' : 'Sync Now'}</span>
        </button>
      </div>
    </header>
  );
};
