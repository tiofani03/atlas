import React from 'react';
import { RefreshCw, Terminal, Circle } from 'lucide-react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../services/api';

interface HeaderProps {
  currentTab: string;
}

export const Header: React.FC<HeaderProps> = ({ currentTab }) => {
  const queryClient = useQueryClient();
  const { data: status } = useQuery({ queryKey: ['status'], queryFn: api.getStatus });
  const { data: syncStatus } = useQuery({ 
    queryKey: ['syncStatus'], 
    queryFn: api.getSyncStatus,
    refetchInterval: (query) => (query.state.data?.is_running ? 1000 : 5000),
  });

  const triggerSyncMutation = useMutation({
    mutationFn: () => api.triggerSync(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['syncStatus'] });
      queryClient.invalidateQueries({ queryKey: ['status'] });
      queryClient.invalidateQueries({ queryKey: ['connectors'] });
    },
  });

  return (
    <header className="h-14 border-b border-zinc-800 bg-zinc-950/80 backdrop-blur-md px-6 flex items-center justify-between sticky top-0 z-30">
      <div className="flex items-center gap-3">
        <h1 className="text-sm font-semibold text-zinc-100 capitalize tracking-wide">
          {currentTab}
        </h1>
        <span className="text-xs px-2 py-0.5 rounded bg-zinc-900 border border-zinc-800 text-zinc-400 font-mono">
          atx v{status?.version || '0.1.0'}
        </span>
      </div>

      <div className="flex items-center gap-4">
        {/* Status Pill */}
        <div className="flex items-center gap-2 text-xs px-2.5 py-1 rounded-full bg-emerald-950/50 border border-emerald-800/40 text-emerald-400">
          <Circle className="w-2 h-2 fill-emerald-500 text-emerald-500 animate-pulse" />
          <span>Localhost Engine Online</span>
        </div>

        {/* CLI Companion Tag */}
        <div className="hidden sm:flex items-center gap-1.5 text-xs text-zinc-500">
          <Terminal className="w-3.5 h-3.5" />
          <span>CLI-First Engine</span>
        </div>

        {/* Global Sync Trigger Button */}
        <button
          onClick={() => triggerSyncMutation.mutate()}
          disabled={syncStatus?.is_running || triggerSyncMutation.isPending}
          className="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 text-white font-medium transition disabled:opacity-50 disabled:cursor-not-allowed shadow-sm shadow-indigo-600/20"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${syncStatus?.is_running ? 'animate-spin' : ''}`} />
          <span>{syncStatus?.is_running ? 'Syncing...' : 'Sync Now'}</span>
        </button>
      </div>
    </header>
  );
};
