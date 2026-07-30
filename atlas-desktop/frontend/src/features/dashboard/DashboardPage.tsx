import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../services/api';
import {
  Database,
  FileCode,
  Plug,
  RefreshCw,
  Clock,
  CheckCircle2,
  AlertCircle,
  HardDrive,
} from 'lucide-react';

export const DashboardPage: React.FC = () => {
  const queryClient = useQueryClient();
  const { data: status, isLoading: statusLoading } = useQuery({
    queryKey: ['status'],
    queryFn: api.getStatus,
  });

  const { data: connectors } = useQuery({
    queryKey: ['connectors'],
    queryFn: api.getConnectors,
  });

  const { data: syncProgress } = useQuery({
    queryKey: ['syncStatus'],
    queryFn: api.getSyncStatus,
  });

  const syncMutation = useMutation({
    mutationFn: (id?: string) => api.triggerSync(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['syncStatus'] });
      queryClient.invalidateQueries({ queryKey: ['status'] });
      queryClient.invalidateQueries({ queryKey: ['connectors'] });
    },
  });

  const formattedDbSize = status
    ? (status.db_size_bytes / (1024 * 1024)).toFixed(2) + ' MB'
    : '0 MB';

  return (
    <div className="p-6 space-y-6">
      {/* Page Header */}
      <div>
        <h2 className="text-xl font-bold text-zinc-100 tracking-tight">Dashboard</h2>
        <p className="text-xs text-zinc-400 mt-1">
          Local engineering knowledge engine health and overview.
        </p>
      </div>

      {/* Primary KPI Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Total Knowledge Objects */}
        <div className="glass-card p-4 rounded-xl flex items-center justify-between">
          <div>
            <p className="text-xs text-zinc-400 font-medium">Indexed Objects</p>
            <p className="text-2xl font-bold text-zinc-100 mt-1">
              {statusLoading ? '...' : status?.total_objects.toLocaleString()}
            </p>
          </div>
          <div className="w-10 h-10 rounded-lg bg-indigo-500/10 border border-indigo-500/20 text-indigo-400 flex items-center justify-center">
            <FileCode className="w-5 h-5" />
          </div>
        </div>

        {/* Database Size */}
        <div className="glass-card p-4 rounded-xl flex items-center justify-between">
          <div>
            <p className="text-xs text-zinc-400 font-medium">Database Size</p>
            <p className="text-2xl font-bold text-zinc-100 mt-1">
              {statusLoading ? '...' : formattedDbSize}
            </p>
          </div>
          <div className="w-10 h-10 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 flex items-center justify-center">
            <Database className="w-5 h-5" />
          </div>
        </div>

        {/* Configured Connectors */}
        <div className="glass-card p-4 rounded-xl flex items-center justify-between">
          <div>
            <p className="text-xs text-zinc-400 font-medium">Active Connectors</p>
            <p className="text-2xl font-bold text-zinc-100 mt-1">
              {statusLoading ? '...' : status?.connectors_count}
            </p>
          </div>
          <div className="w-10 h-10 rounded-lg bg-purple-500/10 border border-purple-500/20 text-purple-400 flex items-center justify-center">
            <Plug className="w-5 h-5" />
          </div>
        </div>

        {/* Atlas Version */}
        <div className="glass-card p-4 rounded-xl flex items-center justify-between">
          <div>
            <p className="text-xs text-zinc-400 font-medium">Engine Version</p>
            <p className="text-2xl font-bold text-zinc-100 mt-1">
              v{status?.version || '0.1.0'}
            </p>
          </div>
          <div className="w-10 h-10 rounded-lg bg-blue-500/10 border border-blue-500/20 text-blue-400 flex items-center justify-center">
            <HardDrive className="w-5 h-5" />
          </div>
        </div>
      </div>

      {/* Sync Status Quick Action Banner */}
      <div className="glass-panel p-5 rounded-xl border border-zinc-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-indigo-600/20 text-indigo-400 flex items-center justify-center shrink-0">
            <RefreshCw className={`w-5 h-5 ${syncProgress?.is_running ? 'animate-spin' : ''}`} />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-zinc-100">
              {syncProgress?.is_running ? 'Synchronization In Progress' : 'Knowledge Engine Synchronization'}
            </h3>
            <p className="text-xs text-zinc-400 mt-0.5">
              {syncProgress?.is_running
                ? `Indexing connector '${syncProgress.current_connector || 'all'}'... (${syncProgress.fetched} items fetched)`
                : `Last completed sync: ${
                    syncProgress?.last_completed_at
                      ? new Date(syncProgress.last_completed_at).toLocaleString()
                      : 'Not synced yet'
                  }`}
            </p>
          </div>
        </div>

        <button
          onClick={() => syncMutation.mutate()}
          disabled={syncProgress?.is_running || syncMutation.isPending}
          className="flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium transition disabled:opacity-50"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${syncProgress?.is_running ? 'animate-spin' : ''}`} />
          <span>{syncProgress?.is_running ? 'Syncing...' : 'Trigger Full Sync'}</span>
        </button>
      </div>

      {/* Configured Connectors Cards Overview */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-zinc-200">Configured Connectors</h3>
          <span className="text-xs text-zinc-500 font-mono">{connectors?.length || 0} configured</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {connectors && connectors.length > 0 ? (
            connectors.map((connector) => (
              <div key={connector.id} className="glass-card p-4 rounded-xl space-y-3">
                <div className="flex items-start justify-between">
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-bold text-zinc-100 capitalize">{connector.id}</span>
                      <span className="text-[10px] px-2 py-0.5 rounded bg-zinc-800 text-zinc-400 font-mono uppercase">
                        {connector.provider}
                      </span>
                    </div>
                    <p className="text-xs text-zinc-400 mt-0.5 truncate max-w-[220px]">
                      {connector.instance_url}
                    </p>
                  </div>
                  <span className="flex items-center gap-1 text-[11px] text-emerald-400 bg-emerald-950/40 px-2 py-0.5 rounded border border-emerald-800/30">
                    <CheckCircle2 className="w-3 h-3" />
                    <span>Configured</span>
                  </span>
                </div>

                <div className="flex items-center justify-between text-xs text-zinc-500 border-t border-zinc-800/60 pt-2">
                  <div className="flex items-center gap-1.5">
                    <Clock className="w-3.5 h-3.5" />
                    <span>
                      {connector.last_synced_at
                        ? new Date(connector.last_synced_at).toLocaleTimeString()
                        : 'Never synced'}
                    </span>
                  </div>
                  <button
                    onClick={() => syncMutation.mutate(connector.id)}
                    disabled={syncProgress?.is_running}
                    className="text-xs text-indigo-400 hover:text-indigo-300 font-medium hover:underline flex items-center gap-1"
                  >
                    <span>Sync</span>
                  </button>
                </div>
              </div>
            ))
          ) : (
            <div className="col-span-2 p-8 border border-dashed border-zinc-800 rounded-xl text-center space-y-2">
              <AlertCircle className="w-6 h-6 text-zinc-500 mx-auto" />
              <p className="text-xs text-zinc-400">No connectors configured yet.</p>
              <p className="text-[11px] text-zinc-500">Go to the Connectors page to setup Jira or Confluence.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
