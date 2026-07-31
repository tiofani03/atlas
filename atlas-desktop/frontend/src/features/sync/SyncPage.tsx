import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../services/api';
import { RefreshCw, CheckCircle2, AlertCircle, Play, Layers } from 'lucide-react';

export const SyncPage: React.FC = () => {
  const queryClient = useQueryClient();

  const { data: syncProgress } = useQuery({
    queryKey: ['syncStatus'],
    queryFn: api.getSyncStatus,
    refetchInterval: (query) => (query.state.data?.is_running ? 1000 : 3000),
  });

  const { data: connectors } = useQuery({
    queryKey: ['connectors'],
    queryFn: api.getConnectors,
  });

  const triggerSyncMutation = useMutation({
    mutationFn: (args?: { connectorId?: string; full?: boolean }) =>
      api.triggerSync(args?.connectorId, args?.full),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['syncStatus'] });
      queryClient.invalidateQueries({ queryKey: ['status'] });
      queryClient.invalidateQueries({ queryKey: ['connectors'] });
    },
  });

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      {/* Header */}
      <div>
        <h2 className="text-xl font-bold text-slate-900 dark:text-zinc-100 tracking-tight">Synchronization Status</h2>
        <p className="text-xs text-slate-500 dark:text-zinc-400 mt-1">
          Monitor real-time connector execution, incremental watermarking, and indexing activity.
        </p>
      </div>

      {/* Progress Card */}
      <div className="glass-panel p-6 rounded-xl border border-slate-200 dark:border-zinc-800 space-y-5 shadow-xs">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 flex items-center justify-center border border-indigo-200 dark:border-indigo-500/30">
              <RefreshCw className={`w-5 h-5 ${syncProgress?.is_running ? 'animate-spin' : ''}`} />
            </div>
            <div>
              <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">
                {syncProgress?.is_running
                  ? `Syncing Connector: ${syncProgress.current_connector || 'All'}`
                  : 'Engine Standby'}
              </h3>
              <p className="text-xs text-slate-500 dark:text-zinc-400 mt-0.5">
                {syncProgress?.is_running
                  ? 'Fetching modified items and updating FTS5 index...'
                  : syncProgress?.last_completed_at
                  ? `Last completed run: ${new Date(syncProgress.last_completed_at).toLocaleString()}`
                  : 'Ready to run synchronization.'}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => triggerSyncMutation.mutate({ full: false })}
              disabled={syncProgress?.is_running || triggerSyncMutation.isPending}
              className="px-3.5 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold transition disabled:opacity-50 flex items-center gap-1.5 shadow-xs"
            >
              <Play className="w-3.5 h-3.5 fill-white" />
              <span>Incremental Sync</span>
            </button>

            <button
              onClick={() => triggerSyncMutation.mutate({ full: true })}
              disabled={syncProgress?.is_running || triggerSyncMutation.isPending}
              className="px-3.5 py-2 rounded-lg bg-slate-100 dark:bg-zinc-800 hover:bg-slate-200 dark:hover:bg-zinc-700 text-slate-800 dark:text-zinc-200 border border-slate-200 dark:border-zinc-700 text-xs font-medium transition disabled:opacity-50 flex items-center gap-1.5 shadow-xs"
            >
              <Layers className="w-3.5 h-3.5 text-slate-500 dark:text-zinc-400" />
              <span>Force Full Sync</span>
            </button>
          </div>
        </div>

        {/* Live Counters */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 border-t border-slate-200 dark:border-zinc-800/80 pt-4">
          <div className="bg-slate-50 dark:bg-zinc-950/60 p-3 rounded-lg border border-slate-200 dark:border-zinc-800/60">
            <span className="text-xs text-slate-500 dark:text-zinc-500 font-medium">Fetched</span>
            <p className="text-lg font-bold text-slate-900 dark:text-zinc-100 mt-0.5">{syncProgress?.fetched || 0}</p>
          </div>
          <div className="bg-slate-50 dark:bg-zinc-950/60 p-3 rounded-lg border border-slate-200 dark:border-zinc-800/60">
            <span className="text-xs text-slate-500 dark:text-zinc-500 font-medium">Inserted</span>
            <p className="text-lg font-bold text-emerald-600 dark:text-emerald-400 mt-0.5">{syncProgress?.inserted || 0}</p>
          </div>
          <div className="bg-slate-50 dark:bg-zinc-950/60 p-3 rounded-lg border border-slate-200 dark:border-zinc-800/60">
            <span className="text-xs text-slate-500 dark:text-zinc-500 font-medium">Updated</span>
            <p className="text-lg font-bold text-blue-600 dark:text-blue-400 mt-0.5">{syncProgress?.updated || 0}</p>
          </div>
          <div className="bg-slate-50 dark:bg-zinc-950/60 p-3 rounded-lg border border-slate-200 dark:border-zinc-800/60">
            <span className="text-xs text-slate-500 dark:text-zinc-500 font-medium">Skipped</span>
            <p className="text-lg font-bold text-slate-600 dark:text-zinc-400 mt-0.5">{syncProgress?.skipped || 0}</p>
          </div>
        </div>

        {/* Error notification if any */}
        {syncProgress?.error && (
          <div className="p-3 rounded-lg bg-rose-50 dark:bg-rose-950/50 border border-rose-200 dark:border-rose-800/50 text-rose-700 dark:text-rose-300 text-xs flex items-center gap-2">
            <AlertCircle className="w-4 h-4 shrink-0 text-rose-600 dark:text-rose-400" />
            <span>{syncProgress.error}</span>
          </div>
        )}
      </div>

      {/* Connectors Run Table */}
      <div className="glass-card p-5 rounded-xl border border-slate-200 dark:border-zinc-800 space-y-4 shadow-xs">
        <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-200">Connector Sync Watermarks</h3>

        <div className="divide-y divide-slate-200 dark:divide-zinc-800/60">
          {connectors && connectors.length > 0 ? (
            connectors.map((c) => (
              <div key={c.id} className="py-3 flex items-center justify-between text-xs">
                <div className="flex items-center gap-3">
                  <div className="w-2 h-2 rounded-full bg-emerald-500" />
                  <div>
                    <span className="font-bold text-slate-900 dark:text-zinc-200">{c.id}</span>
                    <span className="text-slate-500 dark:text-zinc-500 ml-2 font-mono uppercase text-[10px]">({c.provider})</span>
                  </div>
                </div>

                <div className="flex items-center gap-4">
                  <span className="text-slate-500 dark:text-zinc-400">
                    {c.last_synced_at ? new Date(c.last_synced_at).toLocaleString() : 'Never synced'}
                  </span>
                  <button
                    onClick={() => triggerSyncMutation.mutate({ connectorId: c.id, full: false })}
                    disabled={syncProgress?.is_running}
                    className="px-2.5 py-1 rounded bg-slate-100 dark:bg-zinc-800 hover:bg-slate-200 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-300 border border-slate-200 dark:border-zinc-700 font-medium transition disabled:opacity-50"
                  >
                    Sync
                  </button>
                </div>
              </div>
            ))
          ) : (
            <p className="text-xs text-slate-500 dark:text-zinc-500 py-4 text-center">No connectors configured.</p>
          )}
        </div>
      </div>
    </div>
  );
};
