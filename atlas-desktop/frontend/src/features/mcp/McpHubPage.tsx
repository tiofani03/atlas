import React, { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../services/api';
import { ConfigureMcpModal, McpCatalogItem } from './ConfigureMcpModal';
import { McpServerInfo, McpToolSchema, McpTestResult } from '../../types';
import {
  Network,
  Server,
  Plus,
  Play,
  Check,
  X,
  AlertCircle,
  Loader2,
  Copy,
  Trash2,
  Settings2,
  Terminal,
  Key,
  Layers,
  ChevronDown,
  ChevronRight,
  Search,
  RefreshCw,
  Sparkles,
  Code2,
  Wrench,
  CheckCircle2,
  Figma,
  Github,
  Kanban,
  Zap,
  Database,
  FolderGit2,
  Sliders,
  Radio,
} from 'lucide-react';

// Built-in MCP catalog definitions
const BUILTIN_MCP_CATALOG: McpCatalogItem[] = [
  {
    id: 'figma',
    name: 'Figma MCP',
    subtitle: 'Design Tokens & UI Inspection',
    description: 'Connect Figma design tokens, vector components, frames, and comments to all AI coding agents.',
    category: 'design',
    icon: <Figma className="w-5 h-5" />,
    iconBgClass: 'bg-pink-50 dark:bg-pink-600/20 text-pink-600 dark:text-pink-400 border-pink-200 dark:border-pink-500/30',
    defaultCommand: 'npx',
    defaultArgs: ['-y', 'mcp-figma'],
    defaultPrefix: 'figma',
    suggestedEnv: [
      {
        key: 'FIGMA_PERSONAL_ACCESS_TOKEN',
        label: 'Figma Personal Access Token',
        placeholder: 'figd_...',
        description: 'Generated under Figma Account Settings > Personal access tokens',
        required: true,
      },
    ],
    tag: { label: 'Design & UI', color: 'rose' },
  },
  {
    id: 'github',
    name: 'GitHub MCP',
    subtitle: 'Repos, PRs, Commits & Code Search',
    description: 'Inspect repositories, search code, open pull requests, and manage issues across your GitHub orgs.',
    category: 'dev',
    icon: <Github className="w-5 h-5" />,
    iconBgClass: 'bg-slate-100 dark:bg-zinc-800 text-slate-800 dark:text-zinc-200 border-slate-200 dark:border-zinc-700',
    defaultCommand: 'npx',
    defaultArgs: ['-y', '@modelcontextprotocol/server-github'],
    defaultPrefix: 'github',
    suggestedEnv: [
      {
        key: 'GITHUB_PERSONAL_ACCESS_TOKEN',
        label: 'GitHub Personal Access Token',
        placeholder: 'ghp_...',
        description: 'Fine-grained or classic token with repo and read:org permissions',
        required: true,
      },
    ],
    tag: { label: 'Code & Git', color: 'indigo' },
  },
  {
    id: 'clickup',
    name: 'ClickUp MCP',
    subtitle: 'Tasks, Workspaces & Sprints',
    description: 'Manage tasks, sprint backlogs, custom fields, checklists, and workspaces directly from agents.',
    category: 'pm',
    icon: <Kanban className="w-5 h-5" />,
    iconBgClass: 'bg-purple-50 dark:bg-purple-600/20 text-purple-600 dark:text-purple-400 border-purple-200 dark:border-purple-500/30',
    defaultCommand: 'npx',
    defaultArgs: ['-y', '@modelcontextprotocol/server-clickup'],
    defaultPrefix: 'clickup',
    suggestedEnv: [
      {
        key: 'CLICKUP_API_KEY',
        label: 'ClickUp API Key',
        placeholder: 'pk_...',
        description: 'Personal API key from ClickUp Settings > Apps',
        required: true,
      },
    ],
    tag: { label: 'Project Mgmt', color: 'purple' },
  },
  {
    id: 'linear',
    name: 'Linear MCP',
    subtitle: 'Issues, Projects & Cycles',
    description: 'Search issues, view cycle roadmaps, triage bugs, and manage Linear project workflows.',
    category: 'pm',
    icon: <Zap className="w-5 h-5" />,
    iconBgClass: 'bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-500/30',
    defaultCommand: 'npx',
    defaultArgs: ['-y', 'mcp-server-linear'],
    defaultPrefix: 'linear',
    suggestedEnv: [
      {
        key: 'LINEAR_API_KEY',
        label: 'Linear Personal API Key',
        placeholder: 'lin_api_...',
        description: 'Generated in Linear Settings > Account > Security',
        required: true,
      },
    ],
    tag: { label: 'Project Mgmt', color: 'blue' },
  },
  {
    id: 'postgres',
    name: 'PostgreSQL MCP',
    subtitle: 'SQL Querying & Schema Inspect',
    description: 'Direct SQL querying, table schema introspection, and read-only analytical inspections.',
    category: 'data',
    icon: <Database className="w-5 h-5" />,
    iconBgClass: 'bg-blue-50 dark:bg-blue-600/20 text-blue-600 dark:text-blue-400 border-blue-200 dark:border-blue-500/30',
    defaultCommand: 'npx',
    defaultArgs: ['-y', '@modelcontextprotocol/server-postgres', 'postgresql://localhost:5432/mydb'],
    defaultPrefix: 'postgres',
    suggestedEnv: [],
    tag: { label: 'Database', color: 'blue' },
  },
  {
    id: 'filesystem',
    name: 'Filesystem MCP',
    subtitle: 'Directory Sandbox Access',
    description: 'Grant agents scoped, secure file read/write operations within designated project directories.',
    category: 'system',
    icon: <FolderGit2 className="w-5 h-5" />,
    iconBgClass: 'bg-amber-50 dark:bg-amber-600/20 text-amber-600 dark:text-amber-400 border-amber-200 dark:border-amber-500/30',
    defaultCommand: 'npx',
    defaultArgs: ['-y', '@modelcontextprotocol/server-filesystem', '/path/to/project'],
    defaultPrefix: 'fs',
    suggestedEnv: [],
    tag: { label: 'Filesystem', color: 'amber' },
  },
];

interface ToolModalProps {
  isOpen: boolean;
  onClose: () => void;
  serverName: string;
  tools: McpToolSchema[];
}

const DiscoveredToolsModal: React.FC<ToolModalProps> = ({
  isOpen,
  onClose,
  serverName,
  tools,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [expandedTool, setExpandedTool] = useState<string | null>(null);

  if (!isOpen) return null;

  const filteredTools = tools.filter(
    (t) =>
      t.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      (t.description && t.description.toLowerCase().includes(searchTerm.toLowerCase()))
  );

  return (
    <div className="fixed inset-0 z-50 bg-slate-900/50 dark:bg-black/80 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-2xl w-full max-w-3xl p-6 space-y-4 shadow-2xl max-h-[85vh] flex flex-col">
        <div className="flex items-center justify-between border-b border-slate-200 dark:border-zinc-800 pb-3 shrink-0">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-indigo-50 dark:bg-indigo-500/15 text-indigo-600 dark:text-indigo-400 flex items-center justify-center border border-indigo-200 dark:border-indigo-500/30">
              <Wrench className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100 flex items-center gap-2">
                <span>Discovered Tools</span>
                <span className="text-xs px-2 py-0.5 rounded-full bg-indigo-50 dark:bg-indigo-950/60 border border-indigo-200 dark:border-indigo-800/40 text-indigo-700 dark:text-indigo-300 font-mono">
                  {serverName}
                </span>
              </h3>
              <p className="text-xs text-slate-500 dark:text-zinc-400">
                {tools.length} tool{tools.length === 1 ? '' : 's'} registered through this MCP server
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

        {/* Search */}
        <div className="relative shrink-0">
          <Search className="w-4 h-4 absolute left-3 top-2.5 text-slate-400 dark:text-zinc-500" />
          <input
            type="text"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            placeholder="Filter tools by name or description..."
            className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-xl pl-9 pr-3 py-2 text-xs text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-indigo-500"
          />
        </div>

        {/* Tool List */}
        <div className="flex-1 overflow-y-auto space-y-2.5 pr-1 text-xs">
          {filteredTools.length === 0 ? (
            <div className="text-center py-10 text-slate-400 dark:text-zinc-500">
              No matching tools found.
            </div>
          ) : (
            filteredTools.map((tool) => {
              const isExpanded = expandedTool === tool.name;
              const properties = tool.inputSchema?.properties || {};
              const requiredFields = tool.inputSchema?.required || [];
              const propKeys = Object.keys(properties);

              return (
                <div
                  key={tool.name}
                  className="rounded-xl border border-slate-200 dark:border-zinc-800/80 bg-slate-50/50 dark:bg-zinc-950/40 p-3.5 transition hover:border-slate-300 dark:hover:border-zinc-700"
                >
                  <div
                    className="flex items-start justify-between cursor-pointer gap-2"
                    onClick={() => setExpandedTool(isExpanded ? null : tool.name)}
                  >
                    <div className="space-y-1 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-mono font-bold text-indigo-600 dark:text-indigo-400">
                          {tool.name}
                        </span>
                        <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-200 dark:bg-zinc-800 text-slate-600 dark:text-zinc-400">
                          {propKeys.length} param{propKeys.length === 1 ? '' : 's'}
                        </span>
                      </div>
                      {tool.description && (
                        <p className="text-slate-600 dark:text-zinc-400 text-[11px] leading-relaxed">
                          {tool.description}
                        </p>
                      )}
                    </div>
                    <button className="text-slate-400 dark:text-zinc-500 p-1">
                      {isExpanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
                    </button>
                  </div>

                  {isExpanded && (
                    <div className="mt-3 pt-3 border-t border-slate-200 dark:border-zinc-800/80 space-y-2.5">
                      <div className="text-[11px] font-semibold text-slate-700 dark:text-zinc-300 flex items-center justify-between">
                        <span>Input Schema Parameters</span>
                        <span className="text-[10px] font-mono text-slate-400">
                          type: {tool.inputSchema?.type || 'object'}
                        </span>
                      </div>

                      {propKeys.length === 0 ? (
                        <div className="text-[11px] text-slate-400 dark:text-zinc-500 italic">
                          No parameters required for this tool.
                        </div>
                      ) : (
                        <div className="space-y-1.5 bg-white dark:bg-zinc-900 rounded-lg p-2.5 border border-slate-200 dark:border-zinc-800/80">
                          {propKeys.map((key) => {
                            const prop = properties[key];
                            const isReq = requiredFields.includes(key);
                            return (
                              <div
                                key={key}
                                className="flex items-start justify-between py-1 border-b border-slate-100 dark:border-zinc-800/50 last:border-0 gap-2"
                              >
                                <div className="space-y-0.5">
                                  <div className="flex items-center gap-1.5">
                                    <span className="font-mono font-semibold text-slate-800 dark:text-zinc-200">
                                      {key}
                                    </span>
                                    <span className="font-mono text-[10px] text-indigo-500">
                                      ({prop?.type || 'unknown'})
                                    </span>
                                    {isReq ? (
                                      <span className="text-[9px] px-1 py-0.2 rounded bg-rose-50 dark:bg-rose-950/60 text-rose-600 dark:text-rose-400 border border-rose-200 dark:border-rose-900/50 font-mono">
                                        required
                                      </span>
                                    ) : (
                                      <span className="text-[9px] px-1 py-0.2 rounded bg-slate-100 dark:bg-zinc-800 text-slate-500 dark:text-zinc-400 font-mono">
                                        optional
                                      </span>
                                    )}
                                  </div>
                                  {prop?.description && (
                                    <p className="text-[10px] text-slate-500 dark:text-zinc-400">
                                      {prop.description}
                                    </p>
                                  )}
                                </div>
                              </div>
                            );
                          })}
                        </div>
                      )}

                      <details className="text-[10px] font-mono text-slate-500 dark:text-zinc-400 pt-1">
                        <summary className="cursor-pointer hover:text-slate-700 dark:hover:text-zinc-200 select-none">
                          View Raw JSON Schema
                        </summary>
                        <pre className="mt-1.5 p-2 rounded bg-slate-100 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 text-[10px] overflow-x-auto text-slate-800 dark:text-zinc-300">
                          {JSON.stringify(tool.inputSchema || {}, null, 2)}
                        </pre>
                      </details>
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};

export const McpHubPage: React.FC = () => {
  const queryClient = useQueryClient();

  // Modal states
  const [modalTarget, setModalTarget] = useState<{
    catalogItem?: McpCatalogItem | null;
    server?: McpServerInfo | null;
  } | null>(null);

  const [discoveredToolsTarget, setDiscoveredToolsTarget] = useState<{
    serverName: string;
    tools: McpToolSchema[];
  } | null>(null);

  const [deleteConfirmTarget, setDeleteConfirmTarget] = useState<string | null>(null);

  // Search and Category filters
  const [searchQuery, setSearchQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState<string>('all');

  // Testing server spinner state
  const [testingServerName, setTestingServerName] = useState<string | null>(null);
  const [testAlert, setTestAlert] = useState<{
    serverName: string;
    success: boolean;
    message: string;
  } | null>(null);

  // Snippet copy state
  const [activeSnippetTab, setActiveSnippetTab] = useState<'agy' | 'claude' | 'cursor'>('agy');
  const [copiedSnippet, setCopiedSnippet] = useState(false);

  // Fetch configured servers
  const { data: servers, isLoading, refetch } = useQuery({
    queryKey: ['mcpServers'],
    queryFn: api.getMcpServers,
  });

  // Fetch agent configuration snippet
  const { data: snippetData } = useQuery({
    queryKey: ['mcpSnippet'],
    queryFn: api.getMcpSnippet,
  });

  // Mutations
  const deleteMutation = useMutation({
    mutationFn: (name: string) => api.deleteMcpServer(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mcpServers'] });
      queryClient.invalidateQueries({ queryKey: ['status'] });
      setDeleteConfirmTarget(null);
    },
  });

  // Map configured servers by name for instant O(1) lookup
  const configuredMap = useMemo(() => {
    const map = new Map<string, McpServerInfo>();
    if (servers) {
      for (const s of servers) {
        map.set(s.name, s);
      }
    }
    return map;
  }, [servers]);

  // Identify any custom servers not in the built-in catalog
  const customServers = useMemo(() => {
    if (!servers) return [];
    const builtinIds = new Set(BUILTIN_MCP_CATALOG.map((b) => b.id));
    return servers.filter((s) => !builtinIds.has(s.name));
  }, [servers]);

  // Filter categories
  const categories = [
    { id: 'all', label: 'All MCPs' },
    { id: 'configured', label: 'Configured Only' },
    { id: 'design', label: 'Design & UI' },
    { id: 'dev', label: 'Code & Dev' },
    { id: 'pm', label: 'Project Mgmt' },
    { id: 'data', label: 'Database & System' },
  ];

  // Filtering cards based on search and category
  const filteredCatalog = useMemo(() => {
    return BUILTIN_MCP_CATALOG.filter((item) => {
      const isConfigured = configuredMap.has(item.id);

      // Category filter
      if (activeCategory === 'configured' && !isConfigured) return false;
      if (activeCategory === 'design' && item.category !== 'design') return false;
      if (activeCategory === 'dev' && item.category !== 'dev') return false;
      if (activeCategory === 'pm' && item.category !== 'pm') return false;
      if (activeCategory === 'data' && item.category !== 'data' && item.category !== 'system')
        return false;

      // Search filter
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        const matchesName = item.name.toLowerCase().includes(query);
        const matchesDesc = item.description.toLowerCase().includes(query);
        const matchesPrefix = item.defaultPrefix.toLowerCase().includes(query);
        return matchesName || matchesDesc || matchesPrefix;
      }

      return true;
    });
  }, [configuredMap, activeCategory, searchQuery]);

  const filteredCustomServers = useMemo(() => {
    if (activeCategory === 'design' || activeCategory === 'dev' || activeCategory === 'pm') {
      return [];
    }
    return customServers.filter((server) => {
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        const matchesName = server.name.toLowerCase().includes(query);
        const matchesCommand = server.command.toLowerCase().includes(query);
        const matchesPrefix = (server.prefix || '').toLowerCase().includes(query);
        return matchesName || matchesCommand || matchesPrefix;
      }
      return true;
    });
  }, [customServers, activeCategory, searchQuery]);

  const totalConfiguredCount = servers?.length || 0;

  // Handle card test connection
  const handleTestConnection = async (serverName: string) => {
    setTestingServerName(serverName);
    setTestAlert(null);

    try {
      const res = await api.testMcpServer(serverName);
      if (res.success && res.tools && res.tools.length > 0) {
        setTestAlert({
          serverName,
          success: true,
          message: `Success! Discovered ${res.tools.length} tool${
            res.tools.length === 1 ? '' : 's'
          } from ${serverName}.`,
        });
        setDiscoveredToolsTarget({ serverName, tools: res.tools });
      } else if (res.success) {
        setTestAlert({
          serverName,
          success: true,
          message: `Connected successfully to ${serverName}, but no tools were declared.`,
        });
      } else {
        setTestAlert({
          serverName,
          success: false,
          message: res.error || 'Connection failed.',
        });
      }
    } catch (err: unknown) {
      setTestAlert({
        serverName,
        success: false,
        message: (err as Error).message || 'Failed to test server connection.',
      });
    } finally {
      setTestingServerName(null);
    }
  };

  // Copy agent snippet
  const handleCopySnippet = () => {
    let textToCopy = '';
    if (activeSnippetTab === 'agy') {
      textToCopy = JSON.stringify(snippetData?.agy || { mcp: { servers: { atlas: { command: 'atx', args: ['mcp'] } } } }, null, 2);
    } else if (activeSnippetTab === 'claude') {
      textToCopy = JSON.stringify(snippetData?.claude_desktop || { mcpServers: { atlas: { command: 'atx', args: ['mcp'] } } }, null, 2);
    } else {
      textToCopy = JSON.stringify(snippetData?.cursor || { mcpServers: { atlas: { command: 'atx', args: ['mcp'] } } }, null, 2);
    }

    navigator.clipboard.writeText(textToCopy);
    setCopiedSnippet(true);
    setTimeout(() => setCopiedSnippet(false), 2000);
  };

  return (
    <div className="p-6 space-y-8 max-w-7xl mx-auto">
      {/* Hero Header & Stat Overview Banner */}
      <div className="glass-panel p-6 rounded-2xl border border-slate-200 dark:border-zinc-800 shadow-xs space-y-6">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6">
          <div className="space-y-1.5">
            <div className="flex items-center gap-2">
              <span className="p-1.5 rounded-lg bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-500/30">
                <Network className="w-4 h-4" />
              </span>
              <h2 className="text-2xl font-bold text-slate-900 dark:text-zinc-100 tracking-tight">
                MCP Hub & Gateway
              </h2>
            </div>
            <p className="text-xs text-slate-500 dark:text-zinc-400 max-w-2xl leading-relaxed">
              Configure external MCP servers once in Atlas (e.g. Figma, GitHub, ClickUp). All downstream AI agents (Antigravity agy, Claude Desktop, Cursor, Codex) connect to Atlas via <span className="font-mono text-indigo-600 dark:text-indigo-400 font-semibold">atx mcp</span> and instantly inherit all tools.
            </p>
          </div>

          <div className="flex items-center gap-4 shrink-0">
            {/* Quick KPI Badge */}
            <div className="bg-slate-50 dark:bg-zinc-950/80 px-4 py-2.5 rounded-xl border border-slate-200 dark:border-zinc-800 text-center">
              <span className="text-[10px] uppercase font-mono text-slate-400 dark:text-zinc-500 font-semibold block">
                Upstream Servers
              </span>
              <span className="text-base font-bold text-slate-900 dark:text-zinc-100 font-mono">
                {totalConfiguredCount} Active
              </span>
            </div>

            {/* Hub Command Indicator */}
            <div className="hidden sm:flex items-center gap-2 bg-slate-50 dark:bg-zinc-950/80 px-3.5 py-2.5 rounded-xl border border-slate-200 dark:border-zinc-800 font-mono text-xs">
              <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
              <span className="text-slate-500 dark:text-zinc-400">Gateway:</span>
              <span className="font-bold text-indigo-600 dark:text-indigo-400">atx mcp</span>
            </div>

            {/* Add Custom MCP Button */}
            <button
              onClick={() => setModalTarget({ catalogItem: null, server: null })}
              className="px-4 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold transition shadow-md shadow-indigo-500/20 flex items-center gap-2 active:scale-95"
            >
              <Plus className="w-4 h-4" />
              <span>Add Custom MCP</span>
            </button>
          </div>
        </div>

        {/* Global Connection Test Toast Alert */}
        {testAlert && (
          <div
            className={`p-3.5 rounded-xl border flex items-center justify-between gap-3 text-xs ${
              testAlert.success
                ? 'bg-emerald-50 dark:bg-emerald-950/40 text-emerald-800 dark:text-emerald-300 border-emerald-200 dark:border-emerald-800/60'
                : 'bg-rose-50 dark:bg-rose-950/40 text-rose-800 dark:text-rose-300 border-rose-200 dark:border-rose-800/60'
            }`}
          >
            <div className="flex items-center gap-2.5">
              {testAlert.success ? (
                <CheckCircle2 className="w-4 h-4 text-emerald-600 dark:text-emerald-400 shrink-0" />
              ) : (
                <AlertCircle className="w-4 h-4 text-rose-600 dark:text-rose-400 shrink-0" />
              )}
              <span>{testAlert.message}</span>
            </div>
            <button
              onClick={() => setTestAlert(null)}
              className="p-1 text-slate-400 hover:text-slate-600 dark:text-zinc-400"
            >
              <X className="w-3.5 h-3.5" />
            </button>
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
          <div className="relative min-w-[240px]">
            <Search className="w-3.5 h-3.5 text-slate-400 dark:text-zinc-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search MCP servers..."
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-xl pl-8 pr-3 py-1.5 text-xs text-slate-900 dark:text-zinc-100 placeholder-slate-400 dark:placeholder-zinc-500 focus:outline-none focus:border-indigo-500 transition"
            />
          </div>
        </div>
      </div>

      {/* Main MCP Cards Grid */}
      <div className="space-y-4">
        <div className="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-zinc-800/80">
          <div className="flex items-center gap-2">
            <Layers className="w-4 h-4 text-indigo-600 dark:text-indigo-400" />
            <h3 className="text-xs font-bold text-slate-900 dark:text-zinc-200 uppercase tracking-wider">
              MCP Server Catalog & Connectors
            </h3>
          </div>
          <span className="text-[10px] bg-slate-100 dark:bg-zinc-800 text-slate-600 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700 px-2.5 py-0.5 rounded-full font-mono font-semibold">
            {filteredCatalog.length + filteredCustomServers.length} Available
          </span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
          {/* 1. Render Catalog Provider Cards */}
          {filteredCatalog.map((item) => {
            const configuredServer = configuredMap.get(item.id);
            const isConfigured = !!configuredServer;
            const isTesting = testingServerName === item.id;

            return (
              <div
                key={item.id}
                className={`group relative bg-white dark:bg-zinc-900/90 rounded-2xl p-5 border transition-all duration-200 flex flex-col justify-between space-y-4 min-h-[230px] h-full ${
                  isConfigured
                    ? 'border-indigo-300/80 dark:border-indigo-500/40 shadow-2xs hover:shadow-md hover:border-indigo-400 dark:hover:border-indigo-500/60'
                    : 'border-slate-200 dark:border-zinc-800/80 shadow-xs hover:shadow-md hover:border-slate-300 dark:hover:border-zinc-700'
                }`}
              >
                <div>
                  {/* Card Top: Icon, Name, Badge */}
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex items-center gap-3 min-w-0 flex-1">
                      <div
                        className={`w-10 h-10 rounded-xl flex items-center justify-center border shadow-xs shrink-0 transition-transform group-hover:scale-105 ${item.iconBgClass}`}
                      >
                        {item.icon}
                      </div>
                      <div className="min-w-0 flex-1">
                        <h4
                          className="text-sm font-bold text-slate-900 dark:text-zinc-100 flex items-center gap-1.5 truncate"
                          title={item.name}
                        >
                          <span className="truncate">{item.name}</span>
                          {isConfigured && (
                            <span
                              className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse shrink-0"
                              title="Active in Gateway"
                            />
                          )}
                        </h4>
                        <p
                          className="text-[11px] text-slate-500 dark:text-zinc-400 font-medium truncate"
                          title={item.subtitle}
                        >
                          {item.subtitle}
                        </p>
                      </div>
                    </div>

                    <div className="shrink-0 text-right">
                      {isConfigured ? (
                        <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-emerald-700 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50 px-2.5 py-1 rounded-full border border-emerald-200 dark:border-emerald-800/40 whitespace-nowrap">
                          <CheckCircle2 className="w-3.5 h-3.5" />
                          <span>Configured</span>
                        </span>
                      ) : (
                        <span className="text-[10px] text-slate-400 dark:text-zinc-500 font-mono px-2 py-0.5 rounded bg-slate-100 dark:bg-zinc-800/60 border border-slate-200 dark:border-zinc-800 whitespace-nowrap">
                          Not Configured
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Card Body: Configured Details Box vs Description */}
                  {isConfigured && configuredServer ? (
                    <div className="mt-4 space-y-2 text-xs bg-slate-50/80 dark:bg-zinc-950/70 p-3 rounded-xl border border-slate-200/80 dark:border-zinc-800/80">
                      <div className="flex justify-between items-center">
                        <span className="text-slate-500 dark:text-zinc-500 font-medium">Prefix:</span>
                        <span className="text-indigo-600 dark:text-indigo-400 font-mono text-[11px] font-semibold">
                          {configuredServer.prefix || configuredServer.name}__*
                        </span>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-slate-500 dark:text-zinc-500 font-medium">Command:</span>
                        <span
                          className="text-slate-800 dark:text-zinc-300 font-mono text-[10px] truncate max-w-[170px]"
                          title={`${configuredServer.command} ${configuredServer.args.join(' ')}`}
                        >
                          {configuredServer.command} {configuredServer.args.join(' ')}
                        </span>
                      </div>
                      <div className="flex justify-between items-center pt-2 border-t border-slate-200/60 dark:border-zinc-800/60 text-[11px]">
                        <span className="text-slate-400 dark:text-zinc-500">Credentials:</span>
                        <span className="text-slate-700 dark:text-zinc-300 font-mono text-[10px]">
                          {configuredServer.env_keys.length > 0
                            ? `${configuredServer.env_keys.length} Key(s) Saved`
                            : 'No Auth Keys'}
                        </span>
                      </div>
                    </div>
                  ) : (
                    <p className="text-xs text-slate-600 dark:text-zinc-400 mt-3.5 leading-relaxed">
                      {item.description}
                    </p>
                  )}
                </div>

                {/* Card Action Footer */}
                <div className="flex items-center justify-between pt-3 border-t border-slate-100 dark:border-zinc-800/60 mt-auto">
                  <div className="flex items-center gap-1.5">
                    {/* Configure / Edit button: Opens locked modal specifically for this provider */}
                    <button
                      onClick={() =>
                        setModalTarget({
                          catalogItem: item,
                          server: configuredServer || null,
                        })
                      }
                      className="px-3 py-1.5 rounded-xl text-xs font-semibold transition flex items-center gap-1.5 shadow-2xs bg-slate-100 hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-200 border border-slate-200 dark:border-zinc-700"
                    >
                      <Settings2 className="w-3.5 h-3.5 text-slate-400 dark:text-zinc-500" />
                      <span>{isConfigured ? 'Edit' : 'Configure'}</span>
                    </button>

                    {/* Delete server button */}
                    {isConfigured && (
                      <button
                        onClick={() => setDeleteConfirmTarget(item.id)}
                        title="Delete MCP Server"
                        className="p-1.5 rounded-xl bg-slate-100 hover:bg-rose-50 hover:text-rose-600 dark:bg-zinc-800 dark:hover:bg-rose-950/40 dark:hover:text-rose-400 text-slate-500 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700 transition active:scale-95"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    )}
                  </div>

                  {/* Test Connection Button */}
                  {isConfigured && (
                    <button
                      onClick={() => handleTestConnection(item.id)}
                      disabled={isTesting}
                      className="px-3.5 py-1.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold transition flex items-center gap-1.5 disabled:opacity-50 shadow-xs active:scale-95"
                    >
                      {isTesting ? (
                        <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      ) : (
                        <Play className="w-3.5 h-3.5" />
                      )}
                      <span>{isTesting ? 'Testing...' : 'Test'}</span>
                    </button>
                  )}
                </div>
              </div>
            );
          })}

          {/* 2. Render Any Custom Configured Servers as First-Class Cards */}
          {filteredCustomServers.map((server) => {
            const isTesting = testingServerName === server.name;

            return (
              <div
                key={server.name}
                className="group relative bg-white dark:bg-zinc-900/90 rounded-2xl p-5 border border-indigo-300/80 dark:border-indigo-500/40 shadow-2xs hover:shadow-md hover:border-indigo-400 dark:hover:border-indigo-500/60 transition-all duration-200 flex flex-col justify-between space-y-4 min-h-[230px] h-full"
              >
                <div>
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex items-center gap-3 min-w-0 flex-1">
                      <div className="w-10 h-10 rounded-xl flex items-center justify-center border shadow-xs shrink-0 transition-transform group-hover:scale-105 bg-indigo-50 dark:bg-indigo-600/20 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-500/30">
                        <Terminal className="w-5 h-5" />
                      </div>
                      <div className="min-w-0 flex-1">
                        <h4
                          className="text-sm font-bold text-slate-900 dark:text-zinc-100 flex items-center gap-1.5 truncate"
                          title={server.name}
                        >
                          <span className="truncate">{server.name}</span>
                          <span
                            className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse shrink-0"
                            title="Active in Gateway"
                          />
                        </h4>
                        <p className="text-[11px] text-slate-500 dark:text-zinc-400 font-medium truncate">
                          Custom Process MCP
                        </p>
                      </div>
                    </div>

                    <div className="shrink-0 text-right">
                      <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-emerald-700 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/50 px-2.5 py-1 rounded-full border border-emerald-200 dark:border-emerald-800/40 whitespace-nowrap">
                        <CheckCircle2 className="w-3.5 h-3.5" />
                        <span>Configured</span>
                      </span>
                    </div>
                  </div>

                  <div className="mt-4 space-y-2 text-xs bg-slate-50/80 dark:bg-zinc-950/70 p-3 rounded-xl border border-slate-200/80 dark:border-zinc-800/80">
                    <div className="flex justify-between items-center">
                      <span className="text-slate-500 dark:text-zinc-500 font-medium">Prefix:</span>
                      <span className="text-indigo-600 dark:text-indigo-400 font-mono text-[11px] font-semibold">
                        {server.prefix || server.name}__*
                      </span>
                    </div>
                    <div className="flex justify-between items-center">
                      <span className="text-slate-500 dark:text-zinc-500 font-medium">Command:</span>
                      <span
                        className="text-slate-800 dark:text-zinc-300 font-mono text-[10px] truncate max-w-[170px]"
                        title={`${server.command} ${server.args.join(' ')}`}
                      >
                        {server.command} {server.args.join(' ')}
                      </span>
                    </div>
                    <div className="flex justify-between items-center pt-2 border-t border-slate-200/60 dark:border-zinc-800/60 text-[11px]">
                      <span className="text-slate-400 dark:text-zinc-500">Credentials:</span>
                      <span className="text-slate-700 dark:text-zinc-300 font-mono text-[10px]">
                        {server.env_keys.length} Key(s) Saved
                      </span>
                    </div>
                  </div>
                </div>

                <div className="flex items-center justify-between pt-3 border-t border-slate-100 dark:border-zinc-800/60 mt-auto">
                  <div className="flex items-center gap-1.5">
                    <button
                      onClick={() =>
                        setModalTarget({
                          catalogItem: null,
                          server: server,
                        })
                      }
                      className="px-3 py-1.5 rounded-xl text-xs font-semibold transition flex items-center gap-1.5 shadow-2xs bg-slate-100 hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-200 border border-slate-200 dark:border-zinc-700"
                    >
                      <Settings2 className="w-3.5 h-3.5 text-slate-400 dark:text-zinc-500" />
                      <span>Edit</span>
                    </button>

                    <button
                      onClick={() => setDeleteConfirmTarget(server.name)}
                      title="Delete MCP Server"
                      className="p-1.5 rounded-xl bg-slate-100 hover:bg-rose-50 hover:text-rose-600 dark:bg-zinc-800 dark:hover:bg-rose-950/40 dark:hover:text-rose-400 text-slate-500 dark:text-zinc-400 border border-slate-200 dark:border-zinc-700 transition active:scale-95"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>

                  <button
                    onClick={() => handleTestConnection(server.name)}
                    disabled={isTesting}
                    className="px-3.5 py-1.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold transition flex items-center gap-1.5 disabled:opacity-50 shadow-xs active:scale-95"
                  >
                    {isTesting ? (
                      <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    ) : (
                      <Play className="w-3.5 h-3.5" />
                    )}
                    <span>{isTesting ? 'Testing...' : 'Test'}</span>
                  </button>
                </div>
              </div>
            );
          })}

          {/* 3. Add Custom MCP Server Interactive Dashed Card */}
          <div
            onClick={() => setModalTarget({ catalogItem: null, server: null })}
            className="border-2 border-dashed border-slate-300 dark:border-zinc-800 hover:border-indigo-500 dark:hover:border-indigo-500/80 bg-slate-50/40 dark:bg-zinc-950/30 hover:bg-indigo-50/20 dark:hover:bg-indigo-950/10 rounded-2xl p-6 transition-all duration-200 flex flex-col items-center justify-center text-center cursor-pointer min-h-[230px] group shadow-2xs hover:shadow-sm"
          >
            <div className="w-12 h-12 rounded-2xl bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 flex items-center justify-center text-indigo-600 dark:text-indigo-400 mb-3 shadow-xs group-hover:scale-110 transition-transform">
              <Plus className="w-6 h-6" />
            </div>
            <h4 className="text-sm font-bold text-slate-900 dark:text-zinc-100 group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors">
              Add Custom MCP Server
            </h4>
            <p className="text-xs text-slate-500 dark:text-zinc-400 mt-1 max-w-[200px] leading-relaxed">
              Connect any stdio, python, uvx, docker, or npm MCP server
            </p>
          </div>
        </div>
      </div>

      {/* SECTION: Connect Your AI Agents Code Snippet Box */}
      <div className="glass-panel p-6 rounded-2xl border border-slate-200 dark:border-zinc-800 shadow-xs space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <Code2 className="w-4 h-4 text-indigo-600 dark:text-indigo-400" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">
                Connect Your AI Agents
              </h3>
            </div>
            <p className="text-xs text-slate-500 dark:text-zinc-400">
              Downstream agents only need to configure the Atlas MCP command once to access all upstream servers.
            </p>
          </div>

          <div className="flex items-center gap-2">
            {/* Snippet Tabs */}
            <div className="flex bg-slate-100 dark:bg-zinc-800 p-1 rounded-xl">
              <button
                onClick={() => setActiveSnippetTab('agy')}
                className={`px-3 py-1 rounded-lg text-xs font-semibold transition ${
                  activeSnippetTab === 'agy'
                    ? 'bg-white dark:bg-zinc-900 text-indigo-600 dark:text-indigo-400 shadow-xs'
                    : 'text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200'
                }`}
              >
                Antigravity (agy)
              </button>
              <button
                onClick={() => setActiveSnippetTab('claude')}
                className={`px-3 py-1 rounded-lg text-xs font-semibold transition ${
                  activeSnippetTab === 'claude'
                    ? 'bg-white dark:bg-zinc-900 text-indigo-600 dark:text-indigo-400 shadow-xs'
                    : 'text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200'
                }`}
              >
                Claude Desktop
              </button>
              <button
                onClick={() => setActiveSnippetTab('cursor')}
                className={`px-3 py-1 rounded-lg text-xs font-semibold transition ${
                  activeSnippetTab === 'cursor'
                    ? 'bg-white dark:bg-zinc-900 text-indigo-600 dark:text-indigo-400 shadow-xs'
                    : 'text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200'
                }`}
              >
                Cursor
              </button>
            </div>

            <button
              onClick={handleCopySnippet}
              className="px-3 py-1.5 rounded-xl border border-slate-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 hover:bg-slate-50 dark:hover:bg-zinc-800 text-xs font-semibold text-slate-700 dark:text-zinc-300 transition flex items-center gap-1.5 shadow-2xs active:scale-95"
            >
              {copiedSnippet ? (
                <>
                  <Check className="w-3.5 h-3.5 text-emerald-500" />
                  <span className="text-emerald-600 dark:text-emerald-400">Copied!</span>
                </>
              ) : (
                <>
                  <Copy className="w-3.5 h-3.5 text-slate-400" />
                  <span>Copy Config</span>
                </>
              )}
            </button>
          </div>
        </div>

        {/* Code Box */}
        <div className="relative">
          <pre className="p-4 rounded-xl bg-slate-950 text-slate-200 font-mono text-xs overflow-x-auto leading-relaxed border border-slate-800">
            {activeSnippetTab === 'agy' &&
              JSON.stringify(
                snippetData?.agy || {
                  mcp: {
                    servers: {
                      atlas: {
                        command: 'atx',
                        args: ['mcp'],
                      },
                    },
                  },
                },
                null,
                2
              )}
            {activeSnippetTab === 'claude' &&
              JSON.stringify(
                snippetData?.claude_desktop || {
                  mcpServers: {
                    atlas: {
                      command: 'atx',
                      args: ['mcp'],
                    },
                  },
                },
                null,
                2
              )}
            {activeSnippetTab === 'cursor' &&
              JSON.stringify(
                snippetData?.cursor || {
                  mcpServers: {
                    atlas: {
                      command: 'atx',
                      args: ['mcp'],
                    },
                  },
                },
                null,
                2
              )}
          </pre>
        </div>
      </div>

      {/* Provider-Locked / Custom Configure MCP Modal */}
      {modalTarget !== null && (
        <ConfigureMcpModal
          isOpen={true}
          onClose={() => setModalTarget(null)}
          onSuccess={() => {
            queryClient.invalidateQueries({ queryKey: ['mcpServers'] });
            queryClient.invalidateQueries({ queryKey: ['status'] });
            refetch();
          }}
          catalogItem={modalTarget.catalogItem}
          initialServer={modalTarget.server}
        />
      )}

      {/* Discovered Tools Inspector Modal */}
      {discoveredToolsTarget && (
        <DiscoveredToolsModal
          isOpen={true}
          onClose={() => setDiscoveredToolsTarget(null)}
          serverName={discoveredToolsTarget.serverName}
          tools={discoveredToolsTarget.tools}
        />
      )}

      {/* Delete Confirmation Modal */}
      {deleteConfirmTarget && (
        <div className="fixed inset-0 z-50 bg-slate-900/40 dark:bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-2xl w-full max-w-sm p-6 space-y-4 shadow-2xl">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-rose-50 dark:bg-rose-500/20 text-rose-600 dark:text-rose-400 flex items-center justify-center border border-rose-200 dark:border-rose-500/30 shrink-0">
                <Trash2 className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">
                  Delete MCP Server?
                </h3>
                <p className="text-xs text-slate-500 dark:text-zinc-400">
                  Are you sure you want to remove <span className="font-semibold text-slate-800 dark:text-zinc-200">{deleteConfirmTarget}</span> from the MCP Hub?
                </p>
              </div>
            </div>

            <div className="flex justify-end gap-2 pt-2 border-t border-slate-100 dark:border-zinc-800">
              <button
                type="button"
                onClick={() => setDeleteConfirmTarget(null)}
                className="px-3.5 py-1.5 rounded-xl text-slate-600 dark:text-zinc-400 hover:bg-slate-100 dark:hover:bg-zinc-800 text-xs font-semibold transition"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => deleteMutation.mutate(deleteConfirmTarget)}
                disabled={deleteMutation.isPending}
                className="px-3.5 py-1.5 rounded-xl bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold transition flex items-center gap-1.5 disabled:opacity-50 shadow-xs"
              >
                {deleteMutation.isPending && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
                <span>Delete</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
