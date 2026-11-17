import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../services/api';
import { Clock, ExternalLink, Filter } from 'lucide-react';

export const TimelinePage: React.FC = () => {
  const [kindFilter, setKindFilter] = useState('');
  const [providerFilter, setProviderFilter] = useState('');

  const { data, isLoading } = useQuery({
    queryKey: ['timeline', kindFilter, providerFilter],
    queryFn: () => api.getTimeline(kindFilter || undefined, providerFilter || undefined, 100),
  });

  const items = data?.items || [];

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-bold text-slate-900 dark:text-zinc-100 flex items-center gap-2">
            <Clock className="w-5 h-5 text-indigo-500" />
            Engineering Timeline
          </h1>
          <p className="text-xs text-slate-500 dark:text-zinc-400 mt-1">
            Chronological stream of commits, pull requests, tickets, releases, and architecture docs.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Filter className="w-4 h-4 text-slate-400" />
          <select
            value={kindFilter}
            onChange={(e) => setKindFilter(e.target.value)}
            className="px-3 py-1.5 text-xs rounded-xl bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 text-slate-700 dark:text-zinc-200"
          >
            <option value="">All Kinds</option>
            <option value="commit">Commit</option>
            <option value="pull_request">Pull Request</option>
            <option value="issue">Issue</option>
            <option value="ticket">Ticket</option>
            <option value="document">Document</option>
            <option value="release">Release</option>
          </select>
          <input
            type="text"
            value={providerFilter}
            onChange={(e) => setProviderFilter(e.target.value)}
            placeholder="Provider (e.g. github)"
            className="px-3 py-1.5 text-xs rounded-xl bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 text-slate-700 dark:text-zinc-200 w-36"
          />
        </div>
      </div>

      {isLoading ? (
        <div className="text-xs text-slate-500 dark:text-zinc-400 py-8 text-center">Loading timeline...</div>
      ) : items.length === 0 ? (
        <div className="text-xs text-slate-500 dark:text-zinc-400 py-12 text-center border border-dashed border-slate-200 dark:border-zinc-800 rounded-2xl">
          No artifacts found for the selected filters.
        </div>
      ) : (
        <div className="relative pl-6 border-l-2 border-slate-200 dark:border-zinc-800 space-y-5">
          {items.map((item) => {
            const ts = item.created_at || item.updated_at || item.synced_at;
            return (
              <div key={item.id} className="relative bg-white dark:bg-zinc-900/90 border border-slate-200 dark:border-zinc-800 rounded-2xl p-4 shadow-2xs">
                <span className="absolute -left-[31px] top-5 w-3 h-3 rounded-full bg-indigo-500 ring-4 ring-slate-50 dark:ring-zinc-950" />
                <div className="flex items-start justify-between gap-3">
                  <div className="space-y-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded-full bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/40">
                        {item.kind}
                      </span>
                      <span className="text-[10px] font-mono text-slate-500 dark:text-zinc-400">
                        {item.provider} · {item.source_id}
                      </span>
                    </div>
                    <h3 className="text-sm font-semibold text-slate-900 dark:text-zinc-100 truncate">
                      {item.title}
                    </h3>
                    {item.summary && (
                      <p className="text-xs text-slate-600 dark:text-zinc-400 line-clamp-2">{item.summary}</p>
                    )}
                  </div>
                  <div className="flex flex-col items-end gap-1 shrink-0">
                    <span className="text-[11px] font-mono text-slate-400 dark:text-zinc-500">
                      {ts ? new Date(ts).toLocaleString() : ''}
                    </span>
                    {item.source_url && (
                      <a
                        href={item.source_url}
                        target="_blank"
                        rel="noreferrer"
                        className="text-xs text-indigo-600 dark:text-indigo-400 hover:underline flex items-center gap-1"
                      >
                        Open <ExternalLink className="w-3 h-3" />
                      </a>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
