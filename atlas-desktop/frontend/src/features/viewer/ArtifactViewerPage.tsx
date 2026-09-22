import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  Node,
  Edge,
  MarkerType,
  BackgroundVariant,
  useReactFlow,
  ReactFlowProvider,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import {
  Search,
  RefreshCw,
  Sparkles,
  Network,
  Maximize2,
  Filter,
  ArrowRightLeft,
  Loader2,
  AlertCircle,
  Database,
  Table2,
  Info,
  X,
} from 'lucide-react';

import { api } from '../../services/api';
import { GraphNodeData, KnowledgeObject } from '../../types';
import { ArtifactNode, ArtifactNodeType } from './ArtifactNode';
import { ArtifactInspectorDrawer } from './ArtifactInspectorDrawer';
import { getLayoutedElements } from './graphLayout';

const nodeTypes = {
  artifact: ArtifactNode,
};

export interface ArtifactViewerPageProps {
  initialRootId?: string | null;
  onBackToTable?: () => void;
}

const ViewerContent: React.FC<ArtifactViewerPageProps> = ({ initialRootId, onBackToTable }) => {
  const { fitView } = useReactFlow();

  const [nodes, setNodes, onNodesChange] = useNodesState<ArtifactNodeType>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  const [rootId, setRootId] = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<GraphNodeData | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);

  const [isLoading, setIsLoading] = useState(false);
  const [isExpanding, setIsExpanding] = useState(false);
  const [recentSeeds, setRecentSeeds] = useState<GraphNodeData[]>([]);

  // User feedback toasts & alerts
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const [isIsolatedNode, setIsIsolatedNode] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => {
      setToastMessage((cur) => (cur === msg ? null : cur));
    }, 3500);
  };

  // Search state
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<KnowledgeObject[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const searchRef = useRef<HTMLDivElement>(null);

  // Layout & Filter state
  const [direction, setDirection] = useState<'LR' | 'TB'>('LR');
  const [providerFilter, setProviderFilter] = useState<string | null>(null);

  // Close search dropdown on click outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (searchRef.current && !searchRef.current.contains(e.target as HTMLElement)) {
        setIsSearchOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);


  // Search autocomplete debouncing
  useEffect(() => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      setIsSearchOpen(false);
      return;
    }

    const timer = setTimeout(() => {
      setIsSearching(true);
      api.searchObjects(searchQuery.trim(), undefined, undefined, 6)
        .then((res) => {
          setSearchResults(res.items || []);
          setIsSearchOpen(true);
        })
        .catch(() => setSearchResults([]))
        .finally(() => setIsSearching(false));
    }, 250);

    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Load subgraph for root artifact
  const loadGraph = useCallback(
    async (targetId: string, depth = 1) => {
      setIsLoading(true);
      setErrorMessage(null);
      setIsIsolatedNode(false);
      try {
        const resp = await api.getGraph(targetId, depth);
        setRootId(resp.root_id);

        if (resp.nodes.length <= 1 && resp.edges.length === 0) {
          setIsIsolatedNode(true);
        }

        const rawNodes: ArtifactNodeType[] = resp.nodes.map((n) => ({
          id: n.id,
          type: 'artifact',
          data: n,
          position: { x: 0, y: 0 },
        }));

        const rawEdges: Edge[] = resp.edges.map((e) => ({
          id: e.id,
          source: e.source,
          target: e.target,
          label: e.label,
          type: 'smoothstep',
          animated: e.label === 'implements' || e.label === 'contains',
          markerEnd: {
            type: MarkerType.ArrowClosed,
            width: 14,
            height: 14,
            color: '#6366f1',
          },
          style: {
            stroke: '#818cf8',
            strokeWidth: 1.5,
          },
          labelStyle: {
            fill: '#6366f1',
            fontWeight: 600,
            fontSize: 10,
            fontFamily: 'monospace',
          },
          labelBgStyle: {
            fill: '#ffffff',
            fillOpacity: 0.9,
          },
          labelBgPadding: [4, 2] as [number, number],
          labelBgBorderRadius: 4,
        }));

        const layouted = getLayoutedElements(rawNodes, rawEdges, { direction });
        setNodes(layouted.nodes);
        setEdges(layouted.edges);

        // Auto select root node
        const rootNode = resp.nodes.find((n) => n.id === resp.root_id);
        if (rootNode) {
          setSelectedNode(rootNode);
          setIsDrawerOpen(true);
        }

        setTimeout(() => fitView({ padding: 0.25, duration: 400 }), 50);
      } catch (err) {
        console.error('Failed to load graph:', err);
        setErrorMessage(err instanceof Error ? err.message : 'Failed to load graph relationships');
      } finally {
        setIsLoading(false);
      }
    },
    [direction, fitView, setNodes, setEdges]
  );

  // Fetch recent seeds or load initial root on mount
  useEffect(() => {
    let mounted = true;
    if (initialRootId) {
      loadGraph(initialRootId);
      return;
    }

    api.getRecentGraphSeeds()
      .then((seeds) => {
        if (!mounted) return;
        setRecentSeeds(seeds);
        if (seeds.length > 0 && !rootId && !initialRootId) {
          loadGraph(seeds[0].id);
        }
      })
      .catch((err) => {
        console.warn('Could not load recent graph seeds:', err);
      });
    return () => {
      mounted = false;
    };
  }, [initialRootId, loadGraph]);

  // Expand neighbors for a clicked node
  const handleExpandNeighbors = useCallback(
    async (nodeId: string) => {
      setIsExpanding(true);
      setErrorMessage(null);
      try {
        const resp = await api.getGraph(nodeId, 1);

        let addedCount = 0;
        setNodes((prevNodes) => {
          const existingIds = new Set(prevNodes.map((n) => n.id));
          const newNodes: ArtifactNodeType[] = [];

          resp.nodes.forEach((n) => {
            if (!existingIds.has(n.id)) {
              newNodes.push({
                id: n.id,
                type: 'artifact',
                data: n,
                position: { x: 0, y: 0 },
              });
              addedCount++;
            }
          });

          if (addedCount === 0 && resp.edges.length <= edges.length) {
            showToast('All connected neighbors are already shown');
            return prevNodes;
          } else if (addedCount > 0) {
            showToast(`Added +${addedCount} connected artifact(s)`);
          }

          const combinedNodes = [...prevNodes, ...newNodes];

          setEdges((prevEdges) => {
            const existingEdgeIds = new Set(prevEdges.map((e) => e.id));
            const newEdges: Edge[] = [];

            resp.edges.forEach((e) => {
              if (!existingEdgeIds.has(e.id)) {
                newEdges.push({
                  id: e.id,
                  source: e.source,
                  target: e.target,
                  label: e.label,
                  type: 'smoothstep',
                  animated: e.label === 'implements' || e.label === 'contains',
                  markerEnd: {
                    type: MarkerType.ArrowClosed,
                    width: 14,
                    height: 14,
                    color: '#6366f1',
                  },
                  style: {
                    stroke: '#818cf8',
                    strokeWidth: 1.5,
                  },
                  labelStyle: {
                    fill: '#6366f1',
                    fontWeight: 600,
                    fontSize: 10,
                    fontFamily: 'monospace',
                  },
                  labelBgStyle: {
                    fill: '#ffffff',
                    fillOpacity: 0.9,
                  },
                  labelBgPadding: [4, 2] as [number, number],
                  labelBgBorderRadius: 4,
                });
              }
            });

            const combinedEdges = [...prevEdges, ...newEdges];
            const layouted = getLayoutedElements(combinedNodes, combinedEdges, { direction });
            setTimeout(() => fitView({ padding: 0.25, duration: 400 }), 50);
            return layouted.edges;
          });

          const layouted = getLayoutedElements(combinedNodes, edges, { direction });
          return layouted.nodes;
        });
      } catch (err) {
        console.error('Failed to expand neighbors:', err);
        setErrorMessage(err instanceof Error ? err.message : 'Failed to expand neighbors');
      } finally {
        setIsExpanding(false);
      }
    },
    [direction, edges, fitView, setNodes, setEdges]
  );

  // Toggle layout direction
  const handleToggleDirection = () => {
    const nextDir = direction === 'LR' ? 'TB' : 'LR';
    setDirection(nextDir);
    const layouted = getLayoutedElements(nodes, edges, { direction: nextDir });
    setNodes(layouted.nodes);
    setEdges(layouted.edges);
    setTimeout(() => fitView({ padding: 0.25, duration: 300 }), 50);
  };

  // Node click handler
  const handleNodeClick = (_: React.MouseEvent, node: Node) => {
    const data = node.data as unknown as GraphNodeData;
    setSelectedNode(data);
    setIsDrawerOpen(true);
  };

  // Filtered nodes by provider
  const filteredNodes = useMemo(() => {
    if (!providerFilter) return nodes;
    return nodes.map((n) => ({
      ...n,
      hidden: (n.data.provider || '').toLowerCase() !== providerFilter.toLowerCase(),
    }));
  }, [nodes, providerFilter]);

  const uniqueProviders = useMemo(() => {
    const set = new Set<string>();
    nodes.forEach((n) => {
      if (n.data.provider) set.add(n.data.provider.toLowerCase());
    });
    return Array.from(set);
  }, [nodes]);

  return (
    <div className="relative w-full h-[calc(100vh-3.5rem)] flex flex-col bg-slate-50 dark:bg-zinc-950 overflow-hidden select-none">
      {/* Top Toolbar */}
      <header className="h-14 px-4 border-b border-slate-200 dark:border-zinc-800 bg-white/80 dark:bg-zinc-900/80 backdrop-blur-md flex items-center justify-between gap-3 shrink-0 z-10">
        {/* Left: BackToTable & Search Bar */}
        <div className="flex items-center gap-2">
          {onBackToTable && (
            <button
              onClick={onBackToTable}
              className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-semibold border border-slate-200 dark:border-zinc-700 bg-white dark:bg-zinc-800 text-slate-700 dark:text-zinc-200 hover:bg-slate-100 dark:hover:bg-zinc-700 transition shrink-0 shadow-2xs"
              title="Return to Knowledge Table"
            >
              <Table2 className="w-3.5 h-3.5 text-indigo-600 dark:text-indigo-400" />
              <span>Table</span>
            </button>
          )}

          <div ref={searchRef} className="relative w-64 md:w-80">
            <div className="relative">
              <Search className="w-4 h-4 text-slate-400 dark:text-zinc-500 absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                type="text"
                placeholder="Search artifact to visualize (e.g. PAY-100, PR #402)..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onFocus={() => {
                  if (searchResults.length > 0) setIsSearchOpen(true);
                }}
                className="w-full pl-9 pr-8 py-1.5 rounded-lg text-xs bg-slate-100 dark:bg-zinc-800 border border-slate-200 dark:border-zinc-700 text-slate-900 dark:text-zinc-100 placeholder-slate-400 dark:placeholder-zinc-500 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 transition"
              />
              {isSearching && (
                <Loader2 className="w-3.5 h-3.5 text-indigo-500 animate-spin absolute right-3 top-1/2 -translate-y-1/2" />
              )}
            </div>

            {/* Autocomplete Dropdown */}
            {isSearchOpen && searchResults.length > 0 && (
              <div className="absolute left-0 right-0 top-full mt-1 bg-white dark:bg-zinc-900 rounded-xl border border-slate-200 dark:border-zinc-800 shadow-xl overflow-hidden z-30 max-h-72 overflow-y-auto">
                <div className="p-1.5 space-y-0.5">
                  {searchResults.map((item) => (
                    <button
                      key={item.id}
                      onClick={() => {
                        setIsSearchOpen(false);
                        setSearchQuery('');
                        loadGraph(item.id);
                      }}
                      className="w-full text-left p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-zinc-800/80 flex items-center justify-between gap-2 transition"
                    >
                      <div className="min-w-0">
                        <h5 className="text-xs font-semibold text-slate-900 dark:text-zinc-100 truncate">
                          {item.title}
                        </h5>
                        <span className="text-[10px] font-mono text-slate-400 dark:text-zinc-500">
                          {item.source_id || item.id}
                        </span>
                      </div>
                      <span className="text-[9px] font-mono font-bold uppercase px-1.5 py-0.5 rounded bg-slate-100 dark:bg-zinc-800 border border-slate-200 dark:border-zinc-700 text-slate-600 dark:text-zinc-400 shrink-0">
                        {item.provider || 'artifact'}
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Center: Quick Seeds Bar */}
        <div className="hidden lg:flex items-center gap-1.5 overflow-x-auto max-w-md">
          <span className="text-[10px] uppercase font-mono text-slate-400 dark:text-zinc-500 font-semibold shrink-0">
            Recent:
          </span>
          {recentSeeds.slice(0, 4).map((seed) => (
            <button
              key={seed.id}
              onClick={() => loadGraph(seed.id)}
              className={`px-2 py-1 rounded-md text-[11px] font-mono truncate max-w-[130px] border transition ${
                rootId === seed.id
                  ? 'bg-indigo-50 dark:bg-indigo-950/60 border-indigo-300 dark:border-indigo-800 text-indigo-700 dark:text-indigo-300 font-bold'
                  : 'bg-slate-50 dark:bg-zinc-800/50 border-slate-200 dark:border-zinc-800 text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200'
              }`}
              title={seed.title}
            >
              {seed.source_id || seed.title}
            </button>
          ))}
        </div>

        {/* Right: Controls & Filters */}
        <div className="flex items-center gap-2">
          {/* Provider Filter */}
          {uniqueProviders.length > 1 && (
            <div className="flex items-center gap-1 text-xs">
              <Filter className="w-3.5 h-3.5 text-slate-400" />
              <select
                value={providerFilter || ''}
                onChange={(e) => setProviderFilter(e.target.value ? e.target.value : null)}
                className="text-xs bg-slate-100 dark:bg-zinc-800 border border-slate-200 dark:border-zinc-700 rounded-lg px-2 py-1 text-slate-700 dark:text-zinc-300 focus:outline-hidden"
              >
                <option value="">All Providers</option>
                {uniqueProviders.map((p) => (
                  <option key={p} value={p}>
                    {p.toUpperCase()}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Toggle Direction (LR / TB) */}
          <button
            onClick={handleToggleDirection}
            className="flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-medium border border-slate-200 dark:border-zinc-700 bg-white dark:bg-zinc-800 text-slate-700 dark:text-zinc-300 hover:bg-slate-100 dark:hover:bg-zinc-700 transition"
            title={`Switch to ${direction === 'LR' ? 'Top-to-Bottom' : 'Left-to-Right'} layout`}
          >
            <ArrowRightLeft className="w-3.5 h-3.5 text-slate-500" />
            <span className="font-mono text-[11px]">{direction}</span>
          </button>

          {/* Refresh Root Button */}
          {rootId && (
            <button
              onClick={() => loadGraph(rootId)}
              disabled={isLoading}
              className="p-1.5 rounded-lg border border-slate-200 dark:border-zinc-700 bg-white dark:bg-zinc-800 text-slate-700 dark:text-zinc-300 hover:bg-slate-100 dark:hover:bg-zinc-700 transition"
              title="Refresh Graph"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
            </button>
          )}
        </div>
      </header>

      {/* Main Canvas Area */}
      <div className="relative flex-1 w-full h-full">
        {/* Floating Toast Notification */}
        {toastMessage && (
          <div className="absolute top-4 left-1/2 -translate-x-1/2 z-30 px-3.5 py-1.5 rounded-xl bg-slate-900/90 dark:bg-white/90 text-white dark:text-zinc-900 text-xs font-medium shadow-xl backdrop-blur-sm flex items-center gap-2 animate-in fade-in slide-in-from-top-2">
            <Sparkles className="w-3.5 h-3.5 text-indigo-400 dark:text-indigo-600" />
            <span>{toastMessage}</span>
          </div>
        )}

        {/* Isolated Node Alert */}
        {isIsolatedNode && (
          <div className="absolute top-4 left-1/2 -translate-x-1/2 z-20 px-4 py-2 rounded-xl bg-amber-50/90 dark:bg-amber-950/80 border border-amber-200 dark:border-amber-800/60 text-amber-800 dark:text-amber-200 text-xs font-medium shadow-lg backdrop-blur-sm flex items-center gap-2.5">
            <Info className="w-4 h-4 text-amber-600 dark:text-amber-400 shrink-0" />
            <span>Isolated Node: No direct relationships recorded for this artifact yet.</span>
            <button
              onClick={() => setIsIsolatedNode(false)}
              className="text-amber-600 dark:text-amber-400 hover:text-amber-800 p-0.5 ml-1"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* Error Alert */}
        {errorMessage && (
          <div className="absolute top-4 left-1/2 -translate-x-1/2 z-20 px-4 py-2 rounded-xl bg-rose-50/90 dark:bg-rose-950/90 border border-rose-200 dark:border-rose-800 text-rose-800 dark:text-rose-200 text-xs font-medium shadow-lg backdrop-blur-sm flex items-center gap-2.5">
            <AlertCircle className="w-4 h-4 text-rose-600 shrink-0" />
            <span>{errorMessage}</span>
            <button
              onClick={() => setErrorMessage(null)}
              className="text-rose-600 dark:text-rose-400 hover:text-rose-800 p-0.5 ml-1"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {isLoading && (
          <div className="absolute inset-0 bg-white/50 dark:bg-zinc-950/50 backdrop-blur-xs flex items-center justify-center z-20">
            <div className="flex items-center gap-2 px-4 py-2 rounded-xl bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 shadow-xl">
              <Loader2 className="w-4 h-4 text-indigo-600 animate-spin" />
              <span className="text-xs font-semibold text-slate-700 dark:text-zinc-200 font-mono">
                Traversing artifact graph...
              </span>
            </div>
          </div>
        )}

        {nodes.length === 0 && !isLoading ? (
          <div className="h-full flex flex-col items-center justify-center p-6 text-center space-y-4">
            <div className="w-12 h-12 rounded-2xl bg-indigo-50 dark:bg-indigo-950/60 border border-indigo-200 dark:border-indigo-800/40 flex items-center justify-center text-indigo-600 dark:text-indigo-400 shadow-sm">
              <Network className="w-6 h-6" />
            </div>
            <div className="max-w-md space-y-1">
              <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">
                No Artifact Selected
              </h3>
              <p className="text-xs text-slate-500 dark:text-zinc-400">
                Search for an engineering ticket, PR, specification, or document above to visualize its lineage and relationships.
              </p>
            </div>
          </div>
        ) : (
          <ReactFlow
            nodes={filteredNodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onNodeClick={handleNodeClick}
            nodeTypes={nodeTypes}
            minZoom={0.2}
            maxZoom={2.5}
            fitView
            proOptions={{ hideAttribution: true }}
          >
            <Background
              variant={BackgroundVariant.Dots}
              gap={16}
              size={1}
              color="#94a3b8"
              className="opacity-30 dark:opacity-20"
            />
            <Controls className="!bg-white dark:!bg-zinc-900 !border-slate-200 dark:!border-zinc-800 !shadow-lg !rounded-xl overflow-hidden [&>button]:!border-slate-200 dark:[&>button]:!border-zinc-800 dark:[&>button]:!fill-zinc-300" />
            <MiniMap
              className="!bg-white/90 dark:!bg-zinc-900/90 !border-slate-200 dark:!border-zinc-800 !shadow-lg !rounded-xl overflow-hidden"
              nodeColor={(n) => {
                const p = (n.data?.provider as string || '').toLowerCase();
                if (p === 'jira') return '#3b82f6';
                if (p === 'github') return '#a855f7';
                if (p === 'confluence') return '#06b6d4';
                if (p === 'markdown') return '#10b981';
                if (p === 'figma') return '#f59e0b';
                return '#64748b';
              }}
              maskColor="rgba(0, 0, 0, 0.15)"
            />
          </ReactFlow>
        )}

        {/* Inspector Drawer */}
        <ArtifactInspectorDrawer
          node={selectedNode}
          isOpen={isDrawerOpen}
          onClose={() => setIsDrawerOpen(false)}
          onFocusAsRoot={(id) => loadGraph(id)}
          onExpandNeighbors={handleExpandNeighbors}
          isExpanding={isExpanding}
        />
      </div>
    </div>
  );
};

export const ArtifactViewerPage: React.FC<ArtifactViewerPageProps> = (props) => {
  return (
    <ReactFlowProvider>
      <ViewerContent {...props} />
    </ReactFlowProvider>
  );
};
