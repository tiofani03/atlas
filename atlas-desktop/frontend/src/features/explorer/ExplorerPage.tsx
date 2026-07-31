import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../services/api';
import { KnowledgeObject } from '../../types';
import { ObjectDetailModal } from './ObjectDetailModal';
import { Search, ExternalLink, BookOpen, Clock, Tag } from 'lucide-react';

export const ExplorerPage: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');
  const [providerFilter, setProviderFilter] = useState<string>('');
  const [typeFilter, setTypeFilter] = useState<string>('');
  const [selectedObject, setSelectedObject] = useState<KnowledgeObject | null>(null);

  const { data: objects, isLoading } = useQuery({
    queryKey: ['search', searchQuery, typeFilter],
    queryFn: () => api.searchObjects(searchQuery, typeFilter, undefined, 50),
  });

  const filteredObjects = (objects || []).filter((obj) => {
    const provider = obj.provider || obj.source?.provider;
    if (providerFilter && provider !== providerFilter) return false;
    return true;
  });


  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-bold text-slate-900 dark:text-zinc-100 tracking-tight">Knowledge Explorer</h2>
          <p className="text-xs text-slate-500 dark:text-zinc-400 mt-1">
            Read-only full-text search across unified engineering knowledge objects (BM25 FTS5).
          </p>
        </div>

        <span className="text-xs px-2.5 py-1 rounded bg-slate-100 dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 text-slate-600 dark:text-zinc-400 font-mono self-start sm:self-auto font-medium">
          Read-Only Mode
        </span>
      </div>

      {/* Search & Filter Toolbar */}
      <div className="glass-panel p-4 rounded-xl border border-slate-200 dark:border-zinc-800 flex flex-col md:flex-row gap-3 shadow-xs">
        {/* Search Box */}
        <div className="relative flex-1">
          <Search className="w-4 h-4 text-slate-400 dark:text-zinc-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder='Search knowledge (e.g. "payment retry", "Stripe API")...'
            className="w-full bg-white dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg pl-9 pr-4 py-2 text-xs text-slate-900 dark:text-zinc-100 placeholder-slate-400 dark:placeholder-zinc-500 focus:outline-none focus:border-indigo-500 transition"
          />
        </div>

        {/* Filters */}
        <div className="flex items-center gap-2">
          {/* Provider Filter */}
          <select
            value={providerFilter}
            onChange={(e) => setProviderFilter(e.target.value)}
            className="bg-white dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg px-3 py-2 text-xs text-slate-700 dark:text-zinc-300 focus:outline-none focus:border-indigo-500"
          >
            <option value="">All Providers</option>
            <option value="jira">Jira</option>
            <option value="confluence">Confluence</option>
            <option value="github">GitHub</option>
          </select>

          {/* Type Filter */}
          <select
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value)}
            className="bg-white dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg px-3 py-2 text-xs text-slate-700 dark:text-zinc-300 focus:outline-none focus:border-indigo-500"
          >
            <option value="">All Types</option>
            <option value="ticket">Ticket</option>
            <option value="document">Document</option>
            <option value="specification">Specification</option>
            <option value="design">Design</option>
            <option value="component">Component</option>
          </select>
        </div>
      </div>

      {/* Results Table */}
      <div className="glass-card rounded-xl border border-slate-200 dark:border-zinc-800 overflow-hidden shadow-xs">
        <div className="p-4 border-b border-slate-200 dark:border-zinc-800/80 flex items-center justify-between bg-slate-50/50 dark:bg-zinc-950/40">
          <span className="text-xs font-bold text-slate-900 dark:text-zinc-200">
            {isLoading ? 'Searching...' : `Found ${filteredObjects.length} object(s)`}
          </span>
          <span className="text-[11px] text-slate-500 dark:text-zinc-500">Click any row to inspect metadata</span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs text-slate-700 dark:text-zinc-300">
            <thead className="bg-slate-100/80 dark:bg-zinc-950/80 text-slate-500 dark:text-zinc-500 uppercase text-[10px] font-mono tracking-wider border-b border-slate-200 dark:border-zinc-800/60">
              <tr>
                <th className="px-4 py-3">Type</th>
                <th className="px-4 py-3">Title & Summary</th>
                <th className="px-4 py-3">Provider</th>
                <th className="px-4 py-3">Original ID</th>
                <th className="px-4 py-3">Updated</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-200 dark:divide-zinc-800/60">
              {filteredObjects.length > 0 ? (
                filteredObjects.map((obj) => {
                  const kind = obj.kind || obj.object_type || 'artifact';
                  const provider = obj.provider || obj.source?.provider || 'unknown';
                  const sourceId = obj.source_id || obj.source?.original_id || obj.id;
                  const sourceUrl = obj.source_url || obj.source?.web_url || '#';

                  return (
                    <tr
                      key={obj.id}
                      onClick={() => setSelectedObject(obj)}
                      className="hover:bg-slate-50 dark:hover:bg-zinc-900/60 cursor-pointer transition"
                    >
                      <td className="px-4 py-3 shrink-0">
                        <span
                          className={`text-[10px] px-2 py-0.5 rounded font-mono uppercase font-semibold ${
                            kind === 'ticket' || kind === 'issue'
                              ? 'bg-blue-100 dark:bg-blue-950/50 text-blue-700 dark:text-blue-400 border border-blue-200 dark:border-blue-800/30'
                              : kind === 'document' || kind === 'repository'
                              ? 'bg-emerald-100 dark:bg-emerald-950/50 text-emerald-700 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-800/30'
                              : 'bg-purple-100 dark:bg-purple-950/50 text-purple-700 dark:text-purple-400 border border-purple-200 dark:border-purple-800/30'
                          }`}
                        >
                          {kind}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <p className="font-semibold text-slate-900 dark:text-zinc-100 truncate max-w-md">{obj.title}</p>
                        {obj.summary && (
                          <p className="text-[11px] text-slate-500 dark:text-zinc-400 truncate max-w-md mt-0.5">
                            {obj.summary}
                          </p>
                        )}
                      </td>
                      <td className="px-4 py-3 uppercase font-mono text-[11px] text-slate-500 dark:text-zinc-400">
                        {provider}
                      </td>
                      <td className="px-4 py-3 font-mono text-[11px] text-indigo-600 dark:text-indigo-400">
                        {sourceId}
                      </td>
                      <td className="px-4 py-3 text-slate-500 dark:text-zinc-500 whitespace-nowrap">
                        {new Date(obj.updated_at).toLocaleDateString()}
                      </td>
                      <td className="px-4 py-3 text-right shrink-0" onClick={(e) => e.stopPropagation()}>
                        <a
                          href={sourceUrl}
                          target="_blank"
                          rel="noreferrer"
                          className="inline-flex items-center gap-1 text-slate-500 dark:text-zinc-400 hover:text-indigo-600 dark:hover:text-indigo-300 font-medium"
                        >
                          <span>Open</span>
                          <ExternalLink className="w-3 h-3" />
                        </a>
                      </td>
                    </tr>
                  );
                })
              ) : (
                <tr>
                  <td colSpan={6} className="px-4 py-12 text-center text-slate-400 dark:text-zinc-500">
                    <BookOpen className="w-8 h-8 mx-auto mb-2 opacity-50" />
                    <p className="text-xs">No knowledge artifacts match your search query.</p>
                  </td>
                </tr>
              )}
            </tbody>

          </table>
        </div>
      </div>

      {selectedObject && (
        <ObjectDetailModal object={selectedObject} onClose={() => setSelectedObject(null)} />
      )}
    </div>
  );
};
