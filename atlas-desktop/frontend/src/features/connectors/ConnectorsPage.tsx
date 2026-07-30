import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../services/api';
import { ConfigureJiraModal } from './ConfigureJiraModal';
import { ConfigureConfluenceModal } from './ConfigureConfluenceModal';
import {
  Plug,
  RefreshCw,
  Settings2,
  CheckCircle2,
  Clock,
  Github,
  FileText,
  Lock,
} from 'lucide-react';

export const ConnectorsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const [isJiraOpen, setIsJiraOpen] = useState(false);
  const [isConfluenceOpen, setIsConfluenceOpen] = useState(false);

  const { data: connectors, refetch } = useQuery({
    queryKey: ['connectors'],
    queryFn: api.getConnectors,
  });

  const { data: syncProgress } = useQuery({
    queryKey: ['syncStatus'],
    queryFn: api.getSyncStatus,
  });

  const syncMutation = useMutation({
    mutationFn: (id: string) => api.triggerSync(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['syncStatus'] });
      queryClient.invalidateQueries({ queryKey: ['connectors'] });
    },
  });

  const jiraConfig = connectors?.find((c) => c.provider === 'jira');
  const confluenceConfig = connectors?.find((c) => c.provider === 'confluence');

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-xl font-bold text-zinc-100 tracking-tight">Connectors</h2>
        <p className="text-xs text-zinc-400 mt-1">
          Manage integrations and sync pipelines for external knowledge sources.
        </p>
      </div>

      {/* Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Jira Card */}
        <div className="glass-card p-5 rounded-xl space-y-4 border border-zinc-800 flex flex-col justify-between">
          <div>
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-blue-600/20 text-blue-400 flex items-center justify-center font-bold text-lg border border-blue-500/30">
                  J
                </div>
                <div>
                  <h3 className="text-sm font-bold text-zinc-100">Jira Software</h3>
                  <p className="text-xs text-zinc-400">Tickets, Epics, Stories, and Issue Links</p>
                </div>
              </div>
              {jiraConfig ? (
                <span className="flex items-center gap-1 text-[11px] text-emerald-400 bg-emerald-950/40 px-2 py-0.5 rounded border border-emerald-800/30 font-medium">
                  <CheckCircle2 className="w-3 h-3" />
                  <span>Configured</span>
                </span>
              ) : (
                <span className="text-[11px] text-zinc-500 bg-zinc-800/60 px-2 py-0.5 rounded border border-zinc-700/50">
                  Not Configured
                </span>
              )}
            </div>

            {jiraConfig && (
              <div className="mt-4 space-y-2 text-xs bg-zinc-950/60 p-3 rounded-lg border border-zinc-800/80">
                <div className="flex justify-between">
                  <span className="text-zinc-500">Instance URL:</span>
                  <span className="text-zinc-300 font-mono text-[11px] truncate max-w-[200px]">
                    {jiraConfig.instance_url}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-zinc-500">Projects:</span>
                  <span className="text-zinc-300 font-mono text-[11px]">
                    {jiraConfig.projects.join(', ') || 'All Projects'}
                  </span>
                </div>
                <div className="flex justify-between items-center pt-1 border-t border-zinc-800/60 text-zinc-500">
                  <span className="flex items-center gap-1">
                    <Clock className="w-3 h-3 text-zinc-500" />
                    <span>Last Sync:</span>
                  </span>
                  <span className="text-zinc-400">
                    {jiraConfig.last_synced_at
                      ? new Date(jiraConfig.last_synced_at).toLocaleString()
                      : 'Never'}
                  </span>
                </div>
              </div>
            )}
          </div>

          <div className="flex items-center justify-between pt-2 border-t border-zinc-800/60">
            <button
              onClick={() => setIsJiraOpen(true)}
              className="px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-medium transition flex items-center gap-1.5"
            >
              <Settings2 className="w-3.5 h-3.5 text-zinc-400" />
              <span>Configure</span>
            </button>

            {jiraConfig && (
              <button
                onClick={() => syncMutation.mutate(jiraConfig.id)}
                disabled={syncProgress?.is_running}
                className="px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium transition flex items-center gap-1.5 disabled:opacity-50"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${syncProgress?.is_running ? 'animate-spin' : ''}`} />
                <span>Sync Jira</span>
              </button>
            )}
          </div>
        </div>

        {/* Confluence Card */}
        <div className="glass-card p-5 rounded-xl space-y-4 border border-zinc-800 flex flex-col justify-between">
          <div>
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-emerald-600/20 text-emerald-400 flex items-center justify-center font-bold text-lg border border-emerald-500/30">
                  C
                </div>
                <div>
                  <h3 className="text-sm font-bold text-zinc-100">Confluence</h3>
                  <p className="text-xs text-zinc-400">Documentation, Spaces, Specs, and Pages</p>
                </div>
              </div>
              {confluenceConfig ? (
                <span className="flex items-center gap-1 text-[11px] text-emerald-400 bg-emerald-950/40 px-2 py-0.5 rounded border border-emerald-800/30 font-medium">
                  <CheckCircle2 className="w-3 h-3" />
                  <span>Configured</span>
                </span>
              ) : (
                <span className="text-[11px] text-zinc-500 bg-zinc-800/60 px-2 py-0.5 rounded border border-zinc-700/50">
                  Not Configured
                </span>
              )}
            </div>

            {confluenceConfig && (
              <div className="mt-4 space-y-2 text-xs bg-zinc-950/60 p-3 rounded-lg border border-zinc-800/80">
                <div className="flex justify-between">
                  <span className="text-zinc-500">Instance URL:</span>
                  <span className="text-zinc-300 font-mono text-[11px] truncate max-w-[200px]">
                    {confluenceConfig.instance_url}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-zinc-500">Spaces:</span>
                  <span className="text-zinc-300 font-mono text-[11px]">
                    {confluenceConfig.spaces.join(', ') || 'All Spaces'}
                  </span>
                </div>
                <div className="flex justify-between items-center pt-1 border-t border-zinc-800/60 text-zinc-500">
                  <span className="flex items-center gap-1">
                    <Clock className="w-3 h-3 text-zinc-500" />
                    <span>Last Sync:</span>
                  </span>
                  <span className="text-zinc-400">
                    {confluenceConfig.last_synced_at
                      ? new Date(confluenceConfig.last_synced_at).toLocaleString()
                      : 'Never'}
                  </span>
                </div>
              </div>
            )}
          </div>

          <div className="flex items-center justify-between pt-2 border-t border-zinc-800/60">
            <button
              onClick={() => setIsConfluenceOpen(true)}
              className="px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-medium transition flex items-center gap-1.5"
            >
              <Settings2 className="w-3.5 h-3.5 text-zinc-400" />
              <span>Configure</span>
            </button>

            {confluenceConfig && (
              <button
                onClick={() => syncMutation.mutate(confluenceConfig.id)}
                disabled={syncProgress?.is_running}
                className="px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium transition flex items-center gap-1.5 disabled:opacity-50"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${syncProgress?.is_running ? 'animate-spin' : ''}`} />
                <span>Sync Confluence</span>
              </button>
            )}
          </div>
        </div>

        {/* GitHub (Coming Soon) */}
        <div className="glass-card p-5 rounded-xl space-y-4 border border-zinc-800/60 opacity-70">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-purple-600/20 text-purple-400 flex items-center justify-center border border-purple-500/30">
                <Github className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-zinc-300">GitHub</h3>
                <p className="text-xs text-zinc-500">Repositories, Pull Requests, Issues</p>
              </div>
            </div>
            <span className="text-[10px] px-2 py-0.5 rounded bg-purple-950/60 border border-purple-800/40 text-purple-300 font-mono">
              Coming Soon
            </span>
          </div>
          <p className="text-xs text-zinc-500">
            Direct GitHub indexing will normalize pull requests, code reviews, and discussions into Atlas objects.
          </p>
        </div>

        {/* Local Markdown (Coming Soon) */}
        <div className="glass-card p-5 rounded-xl space-y-4 border border-zinc-800/60 opacity-70">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-orange-600/20 text-orange-400 flex items-center justify-center border border-orange-500/30">
                <FileText className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-zinc-300">Markdown Files</h3>
                <p className="text-xs text-zinc-500">Local docs, Obsidian vaults, ADRs</p>
              </div>
            </div>
            <span className="text-[10px] px-2 py-0.5 rounded bg-orange-950/60 border border-orange-800/40 text-orange-300 font-mono">
              Coming Soon
            </span>
          </div>
          <p className="text-xs text-zinc-500">
            Index local Markdown directories and Architecture Decision Records (ADRs) directly into SQLite FTS5.
          </p>
        </div>
      </div>

      {/* Modals */}
      <ConfigureJiraModal
        isOpen={isJiraOpen}
        onClose={() => setIsJiraOpen(false)}
        onSuccess={() => refetch()}
      />
      <ConfigureConfluenceModal
        isOpen={isConfluenceOpen}
        onClose={() => setIsConfluenceOpen(false)}
        onSuccess={() => refetch()}
      />
    </div>
  );
};
