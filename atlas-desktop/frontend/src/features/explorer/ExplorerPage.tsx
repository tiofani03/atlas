import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../services/api';
import { KnowledgeObject } from '../../types';
import { ObjectDetailModal } from './ObjectDetailModal';
import { Search, Filter, ExternalLink, BookOpen, Clock, Tag } from 'lucide-react';

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
    if (providerFilter && obj.source.provider !== providerFilter) return false;
    return true;
  });

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-bold text-zinc-100 tracking-tight">Knowledge Explorer</h2>
          <p className="text-xs text-zinc-400 mt-1">
            Read-only full-text search across unified engineering knowledge objects (BM25 FTS5).
          </p>
        </div>

        <span className="text-xs px-2.5 py-1 rounded bg-zinc-900 border border-zinc-800 text-zinc-400 font-mono self-start sm:self-auto">
          Read-Only Mode
        </span>
      </div>

      {/* Search & Filter Toolbar */}
      <div className="glass-panel p-4 rounded-xl border border-zinc-800 flex flex-col md:flex-row gap-3">
        {/* Search Box */}
        <div className="relative flex-1">
          <Search className="w-4 h-4 text-zinc-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder='Search knowledge (e.g. "payment retry", "Stripe API")...'
            className="w-full bg-zinc-950 border border-zinc-800 rounded-lg pl-9 pr-4 py-2 text-xs text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-indigo-500 transition"
          />
        </div>

        {/* Filters */}
        <div className="flex items-center gap-2">
          {/* Provider Filter */}
          <select
            value={providerFilter}
            onChange={(e) => setProviderFilter(e.target.value)}
            className="bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-xs text-zinc-300 focus:outline-none focus:border-indigo-500"
          >
            <option value="">All Providers</option>
            <option value="jira">Jira</option>
            <option value="confluence">Confluence</option>
          </select>

          {/* Type Filter */}
          <select
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value)}
            className="bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-xs text-zinc-300 focus:outline-none focus:border-indigo-500"
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
      <div className="glass-card rounded-xl border border-zinc-800 overflow-hidden">
        <div className="p-4 border-b border-zinc-800/80 flex items-center justify-between">
          <span className="text-xs font-bold text-zinc-200">
            {isLoading ? 'Searching...' : `Found ${filteredObjects.length} object(s)`}
          </span>
          <span className="text-[11px] text-zinc-500">Click any row to inspect metadata</span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs text-zinc-300">
            <thead className="bg-zinc-950/80 text-zinc-500 uppercase text-[10px] font-mono tracking-wider border-b border-zinc-800/60">
              <tr>
                <th className="px-4 py-3">Type</th>
                <th className="px-4 py-3">Title & Summary</th>
                <th className="px-4 py-3">Provider</th>
                <th className="px-4 py-3">Original ID</th>
                <th className="px-4 py-3">Updated</th>
                <th className="px-4 py-3 text-right">Action</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-800/60">
              {filteredObjects.length > 0 ? (
                filteredObjects.map((obj) => (
                  <tr
                    key={obj.id}
                    onClick={() => setSelectedObject(obj)}
                    className="hover:bg-zinc-800/40 transition cursor-pointer group"
                  >
                    <td className="px-4 py-3 font-mono">
                      <span className="px-2 py-0.5 rounded bg-indigo-950/50 border border-indigo-800/40 text-indigo-300 text-[10px] uppercase">
                        {obj.object_type}
                      </span>
                    </td>
                    <td className="px-4 py-3 max-w-xs sm:max-w-md">
                      <p className="font-semibold text-zinc-100 group-hover:text-indigo-300 transition truncate">
                        {obj.title}
                      </p>
                      {obj.summary && <p className="text-[11px] text-zinc-400 truncate">{obj.summary}</p>}
                    </td>
                    <td className="px-4 py-3 capitalize font-mono text-zinc-400">{obj.source.provider}</td>
                    <td className="px-4 py-3 font-mono text-zinc-300">{obj.source.original_id}</td>
                    <td className="px-4 py-3 text-zinc-400">
                      {new Date(obj.updated_at).toLocaleDateString()}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedObject(obj);
                        }}
                        className="text-xs text-indigo-400 hover:text-indigo-300 font-medium"
                      >
                        Inspect
                      </button>
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={6} className="px-4 py-8 text-center text-zinc-500">
                    {isLoading ? 'Loading knowledge database...' : 'No knowledge objects matched your search filters.'}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Detail Modal */}
      <ObjectDetailModal object={selectedObject} onClose={() => setSelectedObject(null)} />
    </div>
  );
};
