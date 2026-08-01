import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../services/api';
import { KnowledgeObject } from '../../types';
import { ObjectDetailModal } from './ObjectDetailModal';
import {
  Search,
  ExternalLink,
  BookOpen,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  Filter,
  FileText,
} from 'lucide-react';

export const ExplorerPage: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');
  const [providerFilter, setProviderFilter] = useState<string>('');
  const [typeFilter, setTypeFilter] = useState<string>('');
  const [page, setPage] = useState<number>(1);
  const [pageSize, setPageSize] = useState<number>(20);
  const [selectedObject, setSelectedObject] = useState<KnowledgeObject | null>(null);

  // Fetch active connectors to populate provider options dynamically
  const { data: connectors } = useQuery({
    queryKey: ['connectors'],
    queryFn: () => api.getConnectors(),
  });

  // Extract unique active providers + defaults
  const activeProviders = Array.from(
    new Set([
      'markdown',
      'jira',
      'confluence',
      'github',
      ...(connectors || []).map((c) => c.provider),
    ])
  );

  // Paginated Search Query
  const { data: searchResponse, isLoading } = useQuery({
    queryKey: ['search', searchQuery, typeFilter, providerFilter, pageSize, page],
    queryFn: () => api.searchObjects(searchQuery, typeFilter, undefined, pageSize, page, providerFilter),
  });

  const objects = searchResponse?.items || [];
  const total = searchResponse?.total || 0;
  const totalPages = searchResponse?.total_pages || 1;

  const handleSearchChange = (val: string) => {
    setSearchQuery(val);
    setPage(1);
  };

  const handleProviderChange = (val: string) => {
    setProviderFilter(val);
    setPage(1);
  };

  const handleTypeChange = (val: string) => {
    setTypeFilter(val);
    setPage(1);
  };

  const handlePageSizeChange = (val: number) => {
    setPageSize(val);
    setPage(1);
  };

  const startRecord = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const endRecord = Math.min(page * pageSize, total);

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-bold text-slate-900 dark:text-zinc-100 tracking-tight flex items-center gap-2">
            <BookOpen className="w-5 h-5 text-indigo-600 dark:text-indigo-400" />
            <span>Knowledge Explorer</span>
          </h2>
          <p className="text-xs text-slate-500 dark:text-zinc-400 mt-1">
            Full-text search & structured indexing across local Markdown docs, Jira, Confluence, and GitHub.
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
            onChange={(e) => handleSearchChange(e.target.value)}
            placeholder='Search knowledge (e.g. "ADR", "Markdown", "Stripe API")...'
            className="w-full bg-white dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg pl-9 pr-4 py-2 text-xs text-slate-900 dark:text-zinc-100 placeholder-slate-400 dark:placeholder-zinc-500 focus:outline-none focus:border-indigo-500 transition"
          />
        </div>

        {/* Filters */}
        <div className="flex items-center gap-2">
          {/* Provider Filter */}
          <div className="flex items-center gap-1">
            <Filter className="w-3.5 h-3.5 text-slate-400 dark:text-zinc-500" />
            <select
              value={providerFilter}
              onChange={(e) => handleProviderChange(e.target.value)}
              className="bg-white dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg px-3 py-2 text-xs text-slate-700 dark:text-zinc-300 focus:outline-none focus:border-indigo-500"
            >
              <option value="">All Providers ({activeProviders.length})</option>
              {activeProviders.map((p) => (
                <option key={p} value={p}>
                  {p === 'markdown' ? 'Markdown / Local Docs' : p.charAt(0).toUpperCase() + p.slice(1)}
                </option>
              ))}
            </select>
          </div>

          {/* Type Filter */}
          <select
            value={typeFilter}
            onChange={(e) => handleTypeChange(e.target.value)}
            className="bg-white dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-lg px-3 py-2 text-xs text-slate-700 dark:text-zinc-300 focus:outline-none focus:border-indigo-500"
          >
            <option value="">All Object Types</option>
            <option value="ticket">Ticket</option>
            <option value="document">Document</option>
            <option value="specification">Specification</option>
            <option value="design">Design (ADR/RFC)</option>
            <option value="component">Component</option>
          </select>
        </div>
      </div>

      {/* Provider Quick Filter Tabs */}
      <div className="flex items-center gap-1.5 overflow-x-auto pb-1 text-xs">
        <button
          onClick={() => handleProviderChange('')}
          className={`px-3 py-1 rounded-full font-medium transition ${
            providerFilter === ''
              ? 'bg-indigo-600 text-white font-semibold shadow-xs'
              : 'bg-slate-100 dark:bg-zinc-900 text-slate-600 dark:text-zinc-400 hover:bg-slate-200 dark:hover:bg-zinc-800'
          }`}
        >
          All Providers
        </button>
        {activeProviders.map((p) => (
          <button
            key={p}
            onClick={() => handleProviderChange(p)}
            className={`px-3 py-1 rounded-full font-medium transition flex items-center gap-1.5 capitalize ${
              providerFilter === p
                ? 'bg-indigo-600 text-white font-semibold shadow-xs'
                : 'bg-slate-100 dark:bg-zinc-900 text-slate-600 dark:text-zinc-400 hover:bg-slate-200 dark:hover:bg-zinc-800'
            }`}
          >
            {p === 'markdown' && <FileText className="w-3 h-3 text-emerald-500" />}
            <span>{p === 'markdown' ? 'Markdown / Local Docs' : p}</span>
          </button>
        ))}
      </div>

      {/* Results Table */}
      <div className="glass-card rounded-xl border border-slate-200 dark:border-zinc-800 overflow-hidden shadow-xs">
        <div className="p-4 border-b border-slate-200 dark:border-zinc-800/80 flex items-center justify-between bg-slate-50/50 dark:bg-zinc-950/40">
          <span className="text-xs font-bold text-slate-900 dark:text-zinc-200">
            {isLoading ? 'Searching...' : `Showing ${startRecord}–${endRecord} of ${total} object(s)`}
          </span>
          <span className="text-[11px] text-slate-500 dark:text-zinc-500">Click any row to inspect details & metadata</span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs text-slate-700 dark:text-zinc-300">
            <thead className="bg-slate-100/80 dark:bg-zinc-950/80 text-slate-500 dark:text-zinc-500 uppercase text-[10px] font-mono tracking-wider border-b border-slate-200 dark:border-zinc-800/60">
              <tr>
                <th className="px-4 py-3">Type</th>
                <th className="px-4 py-3">Title & Summary</th>
                <th className="px-4 py-3">Provider</th>
                <th className="px-4 py-3">Source / Path</th>
                <th className="px-4 py-3">Updated</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-200 dark:divide-zinc-800/60">
              {objects.length > 0 ? (
                objects.map((obj) => {
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
                              : kind === 'design'
                              ? 'bg-purple-100 dark:bg-purple-950/50 text-purple-700 dark:text-purple-400 border border-purple-200 dark:border-purple-800/30'
                              : 'bg-amber-100 dark:bg-amber-950/50 text-amber-700 dark:text-amber-400 border border-amber-200 dark:border-amber-800/30'
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
                      <td className="px-4 py-3 font-mono text-[11px]">
                        <span
                          className={`px-2 py-0.5 rounded font-semibold uppercase text-[10px] ${
                            provider === 'markdown'
                              ? 'bg-emerald-50 dark:bg-emerald-950/60 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-800/40'
                              : provider === 'jira'
                              ? 'bg-blue-50 dark:bg-blue-950/60 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-800/40'
                              : provider === 'github'
                              ? 'bg-zinc-100 dark:bg-zinc-800 text-zinc-800 dark:text-zinc-200 border border-zinc-300 dark:border-zinc-700'
                              : 'bg-amber-50 dark:bg-amber-950/60 text-amber-600 dark:text-amber-400 border border-amber-200 dark:border-amber-800/40'
                          }`}
                        >
                          {provider}
                        </span>
                      </td>
                      <td className="px-4 py-3 font-mono text-[11px] text-indigo-600 dark:text-indigo-400 max-w-xs truncate">
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
                    <p className="text-xs">No knowledge artifacts match your search query or filters.</p>
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination Bar */}
        <div className="p-3 border-t border-slate-200 dark:border-zinc-800/80 bg-slate-50/80 dark:bg-zinc-950/60 flex flex-col sm:flex-row items-center justify-between gap-3 text-xs">
          {/* Rows per page selector */}
          <div className="flex items-center gap-2 text-slate-600 dark:text-zinc-400">
            <span>Rows per page:</span>
            <select
              value={pageSize}
              onChange={(e) => handlePageSizeChange(Number(e.target.value))}
              className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded px-2 py-1 text-xs text-slate-800 dark:text-zinc-200 focus:outline-none focus:border-indigo-500 font-mono"
            >
              <option value={10}>10</option>
              <option value={20}>20</option>
              <option value={50}>50</option>
              <option value={100}>100</option>
            </select>
          </div>

          {/* Navigation controls */}
          <div className="flex items-center gap-1 text-slate-600 dark:text-zinc-400">
            <span className="mr-2 font-mono text-[11px]">
              Page <strong>{page}</strong> of <strong>{totalPages}</strong> ({total} total)
            </span>

            <button
              onClick={() => setPage(1)}
              disabled={page <= 1}
              className="p-1.5 rounded hover:bg-slate-200 dark:hover:bg-zinc-800 disabled:opacity-30 disabled:hover:bg-transparent transition"
              title="First Page"
            >
              <ChevronsLeft className="w-4 h-4" />
            </button>

            <button
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page <= 1}
              className="p-1.5 rounded hover:bg-slate-200 dark:hover:bg-zinc-800 disabled:opacity-30 disabled:hover:bg-transparent transition"
              title="Previous Page"
            >
              <ChevronLeft className="w-4 h-4" />
            </button>

            <button
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              disabled={page >= totalPages}
              className="p-1.5 rounded hover:bg-slate-200 dark:hover:bg-zinc-800 disabled:opacity-30 disabled:hover:bg-transparent transition"
              title="Next Page"
            >
              <ChevronRight className="w-4 h-4" />
            </button>

            <button
              onClick={() => setPage(totalPages)}
              disabled={page >= totalPages}
              className="p-1.5 rounded hover:bg-slate-200 dark:hover:bg-zinc-800 disabled:opacity-30 disabled:hover:bg-transparent transition"
              title="Last Page"
            >
              <ChevronsRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {selectedObject && (
        <ObjectDetailModal object={selectedObject} onClose={() => setSelectedObject(null)} />
      )}
    </div>
  );
};
