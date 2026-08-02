import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../services/api';
import { ConfigureJiraModal } from './ConfigureJiraModal';
import { ConfigureConfluenceModal } from './ConfigureConfluenceModal';
import { ConfigureGithubModal } from './ConfigureGithubModal';
import { ConfigureMarkdownModal } from './ConfigureMarkdownModal';
import { ConfigureLocalGitModal } from './ConfigureLocalGitModal';
import {
  RefreshCw,
  Settings2,
  CheckCircle2,
  Clock,
  Github,
  FileText,
  FolderGit2,
  Kanban,
  FileSpreadsheet,
  BookOpen,
  GitBranch,
  MessageSquare,
  FileCode,
  Figma,
  Cloud,
  Zap,
  Search,
  Layers,
  Lock,
  HardDrive,
  Trash2,
} from 'lucide-react';

interface ConnectorCardProps {
  name: string;
  subtitle: string;
  description: string;
  icon: React.ReactNode;
  iconBgClass: string;
  tag?: { label: string; color: 'indigo' | 'rose' | 'amber' | 'blue' | 'emerald' | 'purple' | 'slate' };
  isConfigured?: boolean;
  isAvailable?: boolean;
  configuredDetails?: {
    urlLabel: string;
    urlValue: string;
    itemsLabel: string;
    itemsValue: string;
    lastSynced?: string | null;
  };
  onConfigure?: () => void;
  onSync?: () => void;
  onClearData?: () => void;
  isSyncing?: boolean;
  isClearing?: boolean;
}

const TagBadge: React.FC<{ label: string; color: string }> = ({ label, color }) => {
  const colorMap: Record<string, string> = {
    indigo: 'bg-indigo-50 dark:bg-indigo-950/60 border-indigo-200 dark:border-indigo-800/40 text-indigo-700 dark:text-indigo-300',
    rose: 'bg-rose-50 dark:bg-rose-950/60 border-rose-200 dark:border-rose-800/40 text-rose-700 dark:text-rose-300',
    amber: 'bg-indigo-50 dark:bg-indigo-950/60 border-indigo-200 dark:border-indigo-800/40 text-indigo-700 dark:text-indigo-300',
    blue: 'bg-blue-50 dark:bg-blue-950/60 border-blue-200 dark:border-blue-800/40 text-blue-700 dark:text-blue-300',
    emerald: 'bg-emerald-50 dark:bg-emerald-950/60 border-emerald-200 dark:border-emerald-800/40 text-emerald-700 dark:text-emerald-300',
    purple: 'bg-purple-50 dark:bg-purple-950/60 border-purple-200 dark:border-purple-800/40 text-purple-700 dark:text-purple-300',
    slate: 'bg-slate-100 dark:bg-zinc-800 border-slate-200 dark:border-zinc-700 text-slate-500 dark:text-zinc-400',
  };

  return (
    <span className={`text-[10px] px-2.5 py-0.5 rounded-full font-mono font-medium border whitespace-nowrap ${colorMap[color] || colorMap.slate}`}>
      {label}
    </span>
  );
};

const ConnectorCard: React.FC<ConnectorCardProps> = ({
  name,
  subtitle,
  description,
  icon,
  iconBgClass,
  tag,
  isConfigured,
  isAvailable = true,
  configuredDetails,
  onConfigure,
  onSync,
  onClearData,
  isSyncing,
  isClearing,
}) => {
  return (
    <div
      className={`group relative bg-white dark:bg-zinc-900/90 rounded-2xl p-5 border transition-all duration-200 flex flex-col justify-between space-y-4 min-h-[220px] h-full ${
        !isAvailable
          ? 'opacity-60 grayscale-[25%] blur-[0.4px] border-slate-200/80 dark:border-zinc-800/60 pointer-events-none select-none'
          : isConfigured
          ? 'border-indigo-300/80 dark:border-indigo-500/40 shadow-2xs hover:shadow-md hover:border-indigo-400 dark:hover:border-indigo-500/60'
          : 'border-slate-200 dark:border-zinc-800/80 shadow-xs hover:shadow-md hover:border-slate-300 dark:hover:border-zinc-700'
      }`}
    >
      <div>
        {/* Top bar: Icon, Name, and Status Badge */}
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-3 min-w-0 flex-1">
            <div className={`w-10 h-10 rounded-xl flex items-center justify-center border shadow-xs shrink-0 transition-transform ${isAvailable ? 'group-hover:scale-105' : ''} ${iconBgClass}`}>
              {icon}
            </div>
            <div className="min-w-0 flex-1">
              <h4 className="text-sm font-bold text-slate-900 dark:text-zinc-100 flex items-center gap-1.5 truncate" title={name}>
                <span className="truncate">{name}</span>
                {isConfigured && (
                  <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse shrink-0" title="Active Pipeline" />
                )}
              </h4>
              <p className="text-[11px] text-slate-500 dark:text-zinc-400 font-medium truncate" title={subtitle}>{subtitle}</p>
            </div>
          </div>

          <div className="shrink-0 text-right">
            {!isAvailable ? (
              <span className="inline-flex items-center gap-1 text-[10px] font-semibold font-mono text-slate-500 dark:text-zinc-400 bg-slate-100 dark:bg-zinc-800/80 px-2.5 py-0.5 rounded-full border border-slate-200 dark:border-zinc-700 whitespace-nowrap">
                <Lock className="w-3 h-3 text-slate-400 dark:text-zinc-500" />
                <span>Coming Soon</span>
              </span>
            ) : isConfigured ? (
              <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-emerald-700 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50 px-2.5 py-1 rounded-full border border-emerald-200 dark:border-emerald-800/40 whitespace-nowrap">
                <CheckCircle2 className="w-3.5 h-3.5" />
                <span>Configured</span>
              </span>
            ) : tag ? (
              <TagBadge label={tag.label} color={tag.color} />
            ) : (
              <span className="text-[10px] text-slate-400 dark:text-zinc-500 font-mono px-2 py-0.5 rounded bg-slate-100 dark:bg-zinc-800/60 border border-slate-200 dark:border-zinc-800 whitespace-nowrap">
                Not Configured
              </span>
            )}
          </div>
        </div>

        {/* Description or Configured Info Box */}
        {isConfigured && configuredDetails ? (
          <div className="mt-4 space-y-2 text-xs bg-slate-50/80 dark:bg-zinc-950/70 p-3.5 rounded-xl border border-slate-200/80 dark:border-zinc-800/80">
            <div className="flex justify-between items-center">
              <span className="text-slate-500 dark:text-zinc-500 font-medium">{configuredDetails.urlLabel}:</span>
              <span className="text-slate-800 dark:text-zinc-300 font-mono text-[11px] font-semibold truncate max-w-[150px]">
                {configuredDetails.urlValue}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-slate-500 dark:text-zinc-500 font-medium">{configuredDetails.itemsLabel}:</span>
              <span className="text-slate-800 dark:text-zinc-300 font-mono text-[11px] font-semibold truncate max-w-[150px]">
                {configuredDetails.itemsValue}
              </span>
            </div>
            <div className="flex justify-between items-center pt-2 border-t border-slate-200/60 dark:border-zinc-800/60 text-[11px]">
              <span className="text-slate-400 dark:text-zinc-500 flex items-center gap-1">
                <Clock className="w-3 h-3 text-slate-400 dark:text-zinc-500" />
                <span>Last Sync:</span>
              </span>
              <span className="text-slate-700 dark:text-zinc-400 font-medium">
                {configuredDetails.lastSynced
                  ? new Date(configuredDetails.lastSynced).toLocaleString()
                  : 'Never'}
              </span>
            </div>
          </div>
        ) : (
          <p className="text-xs text-slate-600 dark:text-zinc-400 mt-3.5 leading-relaxed">
            {description}
          </p>
        )}
      </div>

      {/* Action Footer */}
      <div className="flex items-center justify-between pt-3 border-t border-slate-100 dark:border-zinc-800/60 mt-auto">
        <div className="flex items-center gap-1.5">
          <button
            onClick={isAvailable ? onConfigure : undefined}
            disabled={!isAvailable}
            className={`px-3 py-1.5 rounded-xl text-xs font-semibold transition flex items-center gap-1.5 shadow-2xs ${
              !isAvailable
                ? 'bg-slate-100 dark:bg-zinc-800/50 text-slate-400 dark:text-zinc-600 border border-slate-200/60 dark:border-zinc-800 cursor-not-allowed pointer-events-none'
                : 'bg-slate-100 hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-200 border border-slate-200 dark:border-zinc-700'
            }`}
          >
            <Settings2 className="w-3.5 h-3.5 text-slate-400 dark:text-zinc-500" />
            <span>{!isAvailable ? 'Not Available' : isConfigured ? 'Edit' : 'Configure'}</span>
          </button>

          {isConfigured && onClearData && isAvailable && (
            <button
              onClick={onClearData}
              disabled={isClearing}
              title="Clear Connector Data"
              className="p-1.5 rounded-xl bg-slate-100 hover:bg-rose-50 hover:text-rose-600 dark:bg-zinc-800 dark:hover:bg-rose-950/40 dark:hover:text-rose-400 text-slate-500 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700 transition active:scale-95"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          )}
        </div>

        {isConfigured && onSync && isAvailable && (
          <button
            onClick={onSync}
            disabled={isSyncing}
            className="px-3.5 py-1.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold transition flex items-center gap-1.5 disabled:opacity-50 shadow-xs active:scale-95"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isSyncing ? 'animate-spin' : ''}`} />
            <span>Sync</span>
          </button>
        )}
      </div>
    </div>
  );
};

export const ConnectorsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const [isJiraOpen, setIsJiraOpen] = useState(false);
  const [isConfluenceOpen, setIsConfluenceOpen] = useState(false);
  const [isGithubOpen, setIsGithubOpen] = useState(false);
  const [isMarkdownOpen, setIsMarkdownOpen] = useState(false);
  const [isLocalGitOpen, setIsLocalGitOpen] = useState(false);

  const [searchQuery, setSearchQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState<string>('all');
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null);

  const { data: connectors, refetch } = useQuery({
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
      queryClient.invalidateQueries({ queryKey: ['connectors'] });
    },
  });

  const deleteConnectorMutation = useMutation({
    mutationFn: ({ id, clearData }: { id: string; clearData: boolean }) => api.deleteConnector(id, clearData),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['connectors'] });
      queryClient.invalidateQueries({ queryKey: ['syncStatus'] });
      queryClient.invalidateQueries({ queryKey: ['status'] });
      setDeleteTarget(null);
    },
  });

  const jiraConfig = connectors?.find((c) => c.provider === 'jira');
  const confluenceConfig = connectors?.find((c) => c.provider === 'confluence');
  const githubConfig = connectors?.find((c) => c.provider === 'github');
  const markdownConfig = connectors?.find((c) => c.provider === 'markdown');
  const localGitConfig = connectors?.find((c) => c.provider === 'local_git');

  const configuredCount = (jiraConfig ? 1 : 0) + (confluenceConfig ? 1 : 0) + (githubConfig ? 1 : 0) + (markdownConfig ? 1 : 0) + (localGitConfig ? 1 : 0);

  const categories = [
    { id: 'all', label: 'All Connectors' },
    { id: 'pm', label: 'Project Management' },
    { id: 'code', label: 'Code & CI/CD' },
    { id: 'comm', label: 'Discussions & APIs' },
    { id: 'docs', label: 'Docs & Vaults' },
  ];

  return (
    <div className="p-6 space-y-8 max-w-7xl mx-auto">
      {/* Hero Header & Stat Overview Banner */}
      <div className="glass-panel p-6 rounded-2xl border border-slate-200 dark:border-zinc-800 shadow-xs space-y-6">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6">
          <div className="space-y-1.5">
            <div className="flex items-center gap-2">
              <span className="p-1.5 rounded-lg bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-500/30">
                <Layers className="w-4 h-4" />
              </span>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-zinc-100 tracking-tight">
                Connectors & Data Sources
              </h2>
            </div>
            <p className="text-xs text-slate-500 dark:text-zinc-400 max-w-2xl leading-relaxed">
              Ingest engineering context from Jira, Confluence, GitHub, Markdown docs, and specifications into a unified local SQLite + FTS5 knowledge graph.
            </p>
          </div>

          <div className="flex items-center gap-4 shrink-0">
            {/* Quick KPI Badge */}
            <div className="bg-slate-50 dark:bg-zinc-950/80 px-4 py-2.5 rounded-xl border border-slate-200 dark:border-zinc-800 text-center">
              <span className="text-[10px] uppercase font-mono text-slate-400 dark:text-zinc-500 font-semibold block">
                Configured
              </span>
              <span className="text-base font-bold text-slate-900 dark:text-zinc-100 font-mono">
                {configuredCount} / 15 Active
              </span>
            </div>

            <button
              onClick={() => syncMutation.mutate(undefined)}
              disabled={syncProgress?.is_running}
              className="px-4 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold transition shadow-md shadow-indigo-500/20 flex items-center gap-2 disabled:opacity-50 active:scale-95"
            >
              <RefreshCw className={`w-4 h-4 ${syncProgress?.is_running ? 'animate-spin' : ''}`} />
              <span>{syncProgress?.is_running ? 'Syncing Pipeline...' : 'Sync All Connectors'}</span>
            </button>
          </div>
        </div>

        {/* Sync Progress Bar Banner */}
        {syncProgress?.is_running && (
          <div className="bg-indigo-50/80 dark:bg-indigo-950/40 p-4 rounded-xl border border-indigo-200 dark:border-indigo-800/60 space-y-2">
            <div className="flex justify-between items-center text-xs font-semibold text-indigo-900 dark:text-indigo-200">
              <span className="flex items-center gap-2">
                <RefreshCw className="w-4 h-4 animate-spin text-indigo-600 dark:text-indigo-400" />
                <span>
                  Syncing {syncProgress.current_connector || 'connectors'}...{' '}
                  <span className="font-normal text-indigo-700 dark:text-indigo-300">
                    {syncProgress.phase ? `(${syncProgress.phase})` : ''}
                  </span>
                </span>
              </span>
              <span className="font-mono font-bold text-indigo-600 dark:text-indigo-400">
                {typeof syncProgress.percentage === 'number'
                  ? `${syncProgress.percentage.toFixed(0)}%`
                  : `${syncProgress.fetched} items`}
              </span>
            </div>
            <div className="w-full h-2.5 bg-indigo-200/80 dark:bg-indigo-900/60 rounded-full overflow-hidden p-0.5">
              <div
                className="h-full bg-indigo-600 dark:bg-indigo-400 rounded-full transition-all duration-300 shadow-2xs"
                style={{ width: `${Math.max(5, syncProgress.percentage || 0)}%` }}
              />
            </div>
          </div>
        )}

        {/* Filter and Search Bar */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pt-4 border-t border-slate-200/80 dark:border-zinc-800/80">
          {/* Category Tabs */}
          <div className="flex items-center gap-1.5 overflow-x-auto pb-1 sm:pb-0">
            {categories.map((cat) => (
              <button
                key={cat.id}
                onClick={() => setActiveCategory(cat.id)}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium transition whitespace-nowrap ${
                  activeCategory === cat.id
                    ? 'bg-indigo-600 text-white font-bold shadow-xs'
                    : 'text-slate-600 dark:text-zinc-400 hover:bg-slate-100 dark:hover:bg-zinc-800/80'
                }`}
              >
                {cat.label}
              </button>
            ))}
          </div>

          {/* Search Box */}
          <div className="relative min-w-[220px]">
            <Search className="w-3.5 h-3.5 text-slate-400 dark:text-zinc-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search connectors..."
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-xl pl-8 pr-3 py-1.5 text-xs text-slate-900 dark:text-zinc-100 placeholder-slate-400 dark:placeholder-zinc-500 focus:outline-none focus:border-indigo-500 transition"
            />
          </div>
        </div>
      </div>

      {/* SECTION 1: Project Management & Tracking */}
      {(activeCategory === 'all' || activeCategory === 'pm') && (
        <div className="space-y-4">
          <div className="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-zinc-800/80">
            <div className="flex items-center gap-2">
              <Kanban className="w-4 h-4 text-blue-600 dark:text-blue-400" />
              <h3 className="text-xs font-bold text-slate-900 dark:text-zinc-200 uppercase tracking-wider">
                Project Management & Issue Tracking
              </h3>
            </div>
            <span className="text-[10px] bg-slate-100 dark:bg-zinc-800 text-slate-600 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700 px-2.5 py-0.5 rounded-full font-mono font-semibold">
              4 Providers
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Jira Card */}
            <ConnectorCard
              name="Jira Software"
              subtitle="Tickets, Epics, & Issues"
              description="Sync Jira issues, custom fields, components, fix versions, and project roadmaps."
              icon={<span className="font-bold text-lg">J</span>}
              iconBgClass="bg-blue-50 dark:bg-blue-600/20 text-blue-600 dark:text-blue-400 border-blue-200 dark:border-blue-500/30"
              isAvailable={true}
              isConfigured={!!jiraConfig}
              configuredDetails={
                jiraConfig
                  ? {
                      urlLabel: 'Instance URL',
                      urlValue: jiraConfig.instance_url,
                      itemsLabel: 'Projects',
                      itemsValue: jiraConfig.projects.join(', ') || 'All Projects',
                      lastSynced: jiraConfig.last_synced_at,
                    }
                  : undefined
              }
              onConfigure={() => setIsJiraOpen(true)}
              onSync={() => jiraConfig && syncMutation.mutate(jiraConfig.id)}
              onClearData={() => jiraConfig && setDeleteTarget({ id: jiraConfig.id, name: 'Jira Software' })}
              isSyncing={syncProgress?.is_running}
              isClearing={deleteConnectorMutation.isPending && deleteTarget?.id === jiraConfig?.id}
            />

            {/* Linear */}
            <ConnectorCard
              name="Linear"
              subtitle="Issues & Cycles"
              description="High-speed GraphQL sync for Linear issues, engineering cycles, roadmaps, and PR attachments."
              icon={<Zap className="w-5 h-5" />}
              iconBgClass="bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-500/30"
              isAvailable={false}
              tag={{ label: 'Popular', color: 'indigo' }}
            />

            {/* Asana */}
            <ConnectorCard
              name="Asana"
              subtitle="Tasks & Projects"
              description="Sync Asana tasks, subtasks, custom field dropdowns, and project milestones."
              icon={<span className="font-bold text-lg">A</span>}
              iconBgClass="bg-rose-50 dark:bg-rose-600/20 text-rose-600 dark:text-rose-400 border-rose-200 dark:border-rose-500/30"
              isAvailable={false}
              tag={{ label: 'Popular', color: 'rose' }}
            />

            {/* ClickUp */}
            <ConnectorCard
              name="ClickUp"
              subtitle="Spaces & Docs"
              description="Sync tasks, sprints, goals, and embedded documents from ClickUp workspaces."
              icon={<span className="font-bold text-lg">CU</span>}
              iconBgClass="bg-pink-50 dark:bg-pink-600/20 text-pink-600 dark:text-pink-400 border-pink-200 dark:border-pink-500/30"
              isAvailable={false}
              tag={{ label: 'Planned', color: 'slate' }}
            />
          </div>
        </div>
      )}

      {/* SECTION 2: Code, Repositories & CI/CD */}
      {(activeCategory === 'all' || activeCategory === 'code') && (
        <div className="space-y-4 pt-2">
          <div className="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-zinc-800/80">
            <div className="flex items-center gap-2">
              <FolderGit2 className="w-4 h-4 text-purple-600 dark:text-purple-400" />
              <h3 className="text-xs font-bold text-slate-900 dark:text-zinc-200 uppercase tracking-wider">
                Code, Repositories & CI/CD
              </h3>
            </div>
            <span className="text-[10px] bg-slate-100 dark:bg-zinc-800 text-slate-600 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700 px-2.5 py-0.5 rounded-full font-mono font-semibold">
              5 Providers
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Local Git Repo */}
            <ConnectorCard
              name="Local Git Repository"
              subtitle="Disk .git & Local Engine"
              description="Read-only local Git metadata extraction, branch resolution, tracked file enumeration, and state sync."
              icon={<HardDrive className="w-5 h-5" />}
              iconBgClass="bg-emerald-50 dark:bg-emerald-600/20 text-emerald-600 dark:text-emerald-400 border-emerald-200 dark:border-emerald-500/30"
              isAvailable={true}
              isConfigured={!!localGitConfig}
              configuredDetails={
                localGitConfig
                  ? {
                      urlLabel: 'Root Path',
                      urlValue: localGitConfig.paths?.join(', ') || localGitConfig.path || './',
                      itemsLabel: 'Repositories',
                      itemsValue: `${localGitConfig.paths?.length || 1} Registered`,
                      lastSynced: localGitConfig.last_synced_at,
                    }
                  : undefined
              }
              onConfigure={() => setIsLocalGitOpen(true)}
              onSync={() => localGitConfig && syncMutation.mutate(localGitConfig.id)}
              onClearData={() => localGitConfig && setDeleteTarget({ id: localGitConfig.id, name: 'Local Git Repository' })}
              isSyncing={syncProgress?.is_running}
              isClearing={deleteConnectorMutation.isPending && deleteTarget?.id === localGitConfig?.id}
            />

            {/* GitHub Card */}
            <ConnectorCard
              name="GitHub"
              subtitle="Repos, PRs, & Issues"
              description="Index pull requests, code reviews, commit messages, and issue discussions."
              icon={<Github className="w-5 h-5" />}
              iconBgClass="bg-purple-50 dark:bg-purple-600/20 text-purple-600 dark:text-purple-400 border-purple-200 dark:border-purple-500/30"
              isAvailable={true}
              isConfigured={!!githubConfig}
              configuredDetails={
                githubConfig
                  ? {
                      urlLabel: 'API URL',
                      urlValue: githubConfig.instance_url,
                      itemsLabel: 'Repos',
                      itemsValue: githubConfig.repos?.join(', ') || 'All Repositories',
                      lastSynced: githubConfig.last_synced_at,
                    }
                  : undefined
              }
              onConfigure={() => setIsGithubOpen(true)}
              onSync={() => githubConfig && syncMutation.mutate(githubConfig.id)}
              onClearData={() => githubConfig && setDeleteTarget({ id: githubConfig.id, name: 'GitHub' })}
              isSyncing={syncProgress?.is_running}
              isClearing={deleteConnectorMutation.isPending && deleteTarget?.id === githubConfig?.id}
            />

            {/* Azure DevOps */}
            <ConnectorCard
              name="Azure DevOps"
              subtitle="Boards & Repos"
              description="Sync Azure Work Items, Git repos, pull requests, and pipeline build logs."
              icon={<Cloud className="w-5 h-5" />}
              iconBgClass="bg-blue-50 dark:bg-blue-500/20 text-blue-600 dark:text-blue-300 border-blue-200 dark:border-blue-400/30"
              isAvailable={false}
              tag={{ label: 'Enterprise', color: 'blue' }}
            />

            {/* GitLab */}
            <ConnectorCard
              name="GitLab"
              subtitle="MRs & Issues"
              description="Index Merge Requests, CI/CD pipelines, and GitLab project wiki pages."
              icon={<GitBranch className="w-5 h-5" />}
              iconBgClass="bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-500/30"
              isAvailable={false}
              tag={{ label: 'Planned', color: 'slate' }}
            />

            {/* Bitbucket */}
            <ConnectorCard
              name="Bitbucket"
              subtitle="Repos & Reviews"
              description="Sync Bitbucket Data Center or Cloud pull requests and code comments."
              icon={<span className="font-bold text-lg">BB</span>}
              iconBgClass="bg-sky-50 dark:bg-sky-600/20 text-sky-600 dark:text-sky-400 border-sky-200 dark:border-sky-500/30"
              isAvailable={false}
              tag={{ label: 'Planned', color: 'slate' }}
            />
          </div>
        </div>
      )}

      {/* SECTION 3: Communication & API Contracts */}
      {(activeCategory === 'all' || activeCategory === 'comm') && (
        <div className="space-y-4 pt-2">
          <div className="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-zinc-800/80">
            <div className="flex items-center gap-2">
              <MessageSquare className="w-4 h-4 text-emerald-600 dark:text-emerald-400" />
              <h3 className="text-xs font-bold text-slate-900 dark:text-zinc-200 uppercase tracking-wider">
                Communication, Discussions & API Contracts
              </h3>
            </div>
            <span className="text-[10px] bg-slate-100 dark:bg-zinc-800 text-slate-600 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700 px-2.5 py-0.5 rounded-full font-mono font-semibold">
              3 Providers
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Slack */}
            <ConnectorCard
              name="Slack Threads"
              subtitle="Engineering Channels"
              description="Index technical decision threads, post-mortems, and architectural discussions from Slack."
              icon={<MessageSquare className="w-5 h-5" />}
              iconBgClass="bg-emerald-50 dark:bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 border-emerald-200 dark:border-emerald-500/30"
              isAvailable={false}
              tag={{ label: 'High Impact', color: 'emerald' }}
            />

            {/* OpenAPI / Postman */}
            <ConnectorCard
              name="OpenAPI / Postman"
              subtitle="API Contract Specs"
              description="Parse OpenAPI v3 YAML/JSON schemas to give AI Agents 100% accurate API contract knowledge."
              icon={<FileCode className="w-5 h-5" />}
              iconBgClass="bg-indigo-50 dark:bg-indigo-500/20 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-500/30"
              isAvailable={false}
              tag={{ label: 'High Impact', color: 'indigo' }}
            />

            {/* Figma Design Specs */}
            <ConnectorCard
              name="Figma Specs"
              subtitle="Design Tokens & UI"
              description="Extract design tokens, component properties, and layout specs directly for UI coding assist."
              icon={<Figma className="w-5 h-5" />}
              iconBgClass="bg-purple-50 dark:bg-purple-500/20 text-purple-600 dark:text-purple-300 border-purple-200 dark:border-purple-400/30"
              isAvailable={false}
              tag={{ label: 'Planned', color: 'slate' }}
            />
          </div>
        </div>
      )}

      {/* SECTION 4: Docs, Vaults & Spreadsheets */}
      {(activeCategory === 'all' || activeCategory === 'docs') && (
        <div className="space-y-4 pt-2">
          <div className="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-zinc-800/80">
            <div className="flex items-center gap-2">
              <BookOpen className="w-4 h-4 text-teal-600 dark:text-teal-400" />
              <h3 className="text-xs font-bold text-slate-900 dark:text-zinc-200 uppercase tracking-wider">
                Docs, Knowledge Vaults & Spreadsheets
              </h3>
            </div>
            <span className="text-[10px] bg-slate-100 dark:bg-zinc-800 text-slate-600 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700 px-2.5 py-0.5 rounded-full font-mono font-semibold">
              4 Providers
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Confluence Card */}
            <ConnectorCard
              name="Confluence"
              subtitle="Documentation & Spaces"
              description="Ingest Confluence space pages, PRDs, engineering guides, and decision records."
              icon={<span className="font-bold text-lg">C</span>}
              iconBgClass="bg-emerald-50 dark:bg-emerald-600/20 text-emerald-600 dark:text-emerald-400 border-emerald-200 dark:border-emerald-500/30"
              isAvailable={true}
              isConfigured={!!confluenceConfig}
              configuredDetails={
                confluenceConfig
                  ? {
                      urlLabel: 'Instance URL',
                      urlValue: confluenceConfig.instance_url,
                      itemsLabel: 'Spaces',
                      itemsValue: confluenceConfig.spaces.join(', ') || 'All Spaces',
                      lastSynced: confluenceConfig.last_synced_at,
                    }
                  : undefined
              }
              onConfigure={() => setIsConfluenceOpen(true)}
              onSync={() => confluenceConfig && syncMutation.mutate(confluenceConfig.id)}
              onClearData={() => confluenceConfig && setDeleteTarget({ id: confluenceConfig.id, name: 'Confluence' })}
              isSyncing={syncProgress?.is_running}
              isClearing={deleteConnectorMutation.isPending && deleteTarget?.id === confluenceConfig?.id}
            />

            {/* Notion */}
            <ConnectorCard
              name="Notion Workspace"
              subtitle="Databases & PRD Pages"
              description="Direct Notion API integration to ingest engineering wikis and task databases."
              icon={<span className="font-bold text-lg">N</span>}
              iconBgClass="bg-teal-50 dark:bg-teal-600/20 text-teal-600 dark:text-teal-400 border-teal-200 dark:border-teal-500/30"
              isAvailable={false}
              tag={{ label: 'Popular', color: 'emerald' }}
            />

            {/* Local Markdown */}
            <ConnectorCard
              name="Local Markdown & ADRs"
              subtitle="Obsidian Vaults & Specs"
              description="Index local Markdown files, Architecture Decision Records (ADRs), and Obsidian vaults."
              icon={<FileText className="w-5 h-5" />}
              iconBgClass="bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-500/30"
              isAvailable={true}
              isConfigured={!!markdownConfig}
              configuredDetails={
                markdownConfig
                  ? {
                      urlLabel: 'Directory Path',
                      urlValue: markdownConfig.path || './docs',
                      itemsLabel: 'Glob Patterns',
                      itemsValue: markdownConfig.glob_patterns?.join(', ') || '*.md',
                      lastSynced: markdownConfig.last_synced_at,
                    }
                  : undefined
              }
              onConfigure={() => setIsMarkdownOpen(true)}
              onSync={() => markdownConfig && syncMutation.mutate(markdownConfig.id)}
              onClearData={() => markdownConfig && setDeleteTarget({ id: markdownConfig.id, name: 'Local Markdown & ADRs' })}
              isSyncing={syncProgress?.is_running}
              isClearing={deleteConnectorMutation.isPending && deleteTarget?.id === markdownConfig?.id}
            />

            {/* Spreadsheets */}
            <ConnectorCard
              name="Spreadsheets & CSV"
              subtitle="Google Sheets & Excel"
              description="Parse tabular data, requirement matrices, and technical specifications from spreadsheets."
              icon={<FileSpreadsheet className="w-5 h-5" />}
              iconBgClass="bg-emerald-50 dark:bg-emerald-600/20 text-emerald-600 dark:text-emerald-400 border-emerald-200 dark:border-emerald-500/30"
              isAvailable={false}
              tag={{ label: 'Planned', color: 'slate' }}
            />
          </div>
        </div>
      )}

      {/* Modals */}
      <ConfigureJiraModal
        isOpen={isJiraOpen}
        onClose={() => setIsJiraOpen(false)}
        onSuccess={() => refetch()}
        initialConfig={jiraConfig ? {
          id: jiraConfig.id,
          instance_url: jiraConfig.instance_url,
          email: jiraConfig.email,
          projects: jiraConfig.projects,
        } : undefined}
      />
      <ConfigureConfluenceModal
        isOpen={isConfluenceOpen}
        onClose={() => setIsConfluenceOpen(false)}
        onSuccess={() => refetch()}
        initialConfig={confluenceConfig ? {
          id: confluenceConfig.id,
          instance_url: confluenceConfig.instance_url,
          email: confluenceConfig.email,
          spaces: confluenceConfig.spaces,
        } : undefined}
      />
      <ConfigureGithubModal
        isOpen={isGithubOpen}
        onClose={() => setIsGithubOpen(false)}
        onSuccess={() => refetch()}
        initialConfig={githubConfig ? {
          id: githubConfig.id,
          instance_url: githubConfig.instance_url,
          repos: githubConfig.repos,
        } : undefined}
      />
      <ConfigureMarkdownModal
        isOpen={isMarkdownOpen}
        onClose={() => setIsMarkdownOpen(false)}
        onSuccess={() => refetch()}
        initialConfig={markdownConfig ? {
          id: markdownConfig.id,
          path: markdownConfig.path,
          glob_patterns: markdownConfig.glob_patterns,
        } : undefined}
      />
      <ConfigureLocalGitModal
        isOpen={isLocalGitOpen}
        onClose={() => setIsLocalGitOpen(false)}
        onSuccess={() => refetch()}
        initialConfig={localGitConfig ? {
          id: localGitConfig.id,
          path: localGitConfig.path,
          paths: localGitConfig.paths,
        } : undefined}
      />

      {/* Delete / Clear Connector Confirmation Modal */}
      {deleteTarget && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs animate-in fade-in duration-200">
          <div className="bg-white dark:bg-zinc-900 rounded-2xl p-6 max-w-md w-full border border-slate-200 dark:border-zinc-800 shadow-xl space-y-4">
            <div className="flex items-center gap-3 text-rose-600 dark:text-rose-400">
              <div className="p-2 rounded-xl bg-rose-50 dark:bg-rose-950/60 border border-rose-200 dark:border-rose-800/40">
                <Trash2 className="w-5 h-5" />
              </div>
              <h3 className="text-lg font-bold text-slate-900 dark:text-zinc-100">
                Delete Connector & Data
              </h3>
            </div>
            
            <p className="text-xs text-slate-600 dark:text-zinc-400 leading-relaxed">
              Are you sure you want to delete connector <strong className="text-slate-900 dark:text-zinc-200">{deleteTarget.name}</strong> (<code className="font-mono text-indigo-600 dark:text-indigo-400">{deleteTarget.id}</code>)? This will remove its configuration and clear all its synchronized artifacts from the local database.
            </p>

            <div className="flex items-center justify-end gap-2 pt-2">
              <button
                onClick={() => setDeleteTarget(null)}
                className="px-4 py-2 rounded-xl text-xs font-semibold bg-slate-100 hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-300 border border-slate-200 dark:border-zinc-700 transition"
              >
                Cancel
              </button>
              <button
                onClick={() => deleteConnectorMutation.mutate({ id: deleteTarget.id, clearData: true })}
                disabled={deleteConnectorMutation.isPending}
                className="px-4 py-2 rounded-xl text-xs font-bold bg-rose-600 hover:bg-rose-500 text-white transition flex items-center gap-1.5 disabled:opacity-50 shadow-md shadow-rose-500/20"
              >
                {deleteConnectorMutation.isPending ? 'Deleting...' : 'Delete & Clear Data'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default ConnectorsPage;
