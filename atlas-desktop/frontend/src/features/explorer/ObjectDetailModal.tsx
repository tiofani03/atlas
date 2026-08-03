import React, { useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { X, ExternalLink, Tag, Link2, FileCode, Layers, Loader2, AlertCircle } from 'lucide-react';
import { api } from '../../services/api';
import { KnowledgeObject } from '../../types';

interface ModalProps {
  object: KnowledgeObject;
  onClose: () => void;
  onNavigate?: (id: string) => void;
}

export const ObjectDetailModal: React.FC<ModalProps> = ({ object, onClose, onNavigate }) => {
  const id = object.id || object.source_id || '';

  // Hydrate the full artifact from the backend so body, metadata and
  // relationships reflect the latest indexed state.
  const { data, isLoading, isError, error } = useQuery({
    queryKey: ['object', id],
    queryFn: () => api.getObjectById(id),
    enabled: !!id,
  });

  const resolved = data ?? object;

  // Keyboard navigation: Escape closes, arrows move through relationships.
  useEffect(() => {
    if (!onNavigate) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [onClose, onNavigate]);

  const kind = resolved.kind || resolved.object_type || 'artifact';
  const provider = resolved.provider || resolved.source?.provider || 'unknown';
  const sourceId = resolved.source_id || resolved.source?.original_id || resolved.id;
  const sourceUrl = resolved.source_url || resolved.source?.web_url || '#';
  const textContent = resolved.body || resolved.content || 'No text content extracted.';
  const rawMetadata = resolved.metadata || resolved.source_metadata || {};

  return (
    <div
      className="fixed inset-0 z-50 bg-slate-900/40 dark:bg-black/70 backdrop-blur-sm flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-label={`${kind} detail`}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-xl w-full max-w-3xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="p-5 border-b border-slate-200 dark:border-zinc-800 flex items-start justify-between bg-slate-50 dark:bg-zinc-950/40">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-indigo-50 dark:bg-indigo-950/60 border border-indigo-200 dark:border-indigo-800/40 text-indigo-700 dark:text-indigo-300 font-bold">
                {kind}
              </span>
              <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-slate-100 dark:bg-zinc-800 border border-slate-200 dark:border-zinc-700 text-slate-600 dark:text-zinc-400">
                {provider}
              </span>
              {isLoading && (
                <span className="flex items-center gap-1 text-[10px] text-slate-400 dark:text-zinc-500 font-mono">
                  <Loader2 className="w-3 h-3 animate-spin" />
                  refreshing
                </span>
              )}
            </div>
            <h3 className="text-base font-bold text-slate-900 dark:text-zinc-100">{resolved.title}</h3>
            {resolved.summary && <p className="text-xs text-slate-500 dark:text-zinc-400">{resolved.summary}</p>}
          </div>

          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 dark:text-zinc-400 dark:hover:text-zinc-200" aria-label="Close">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 overflow-y-auto space-y-5 text-xs">
          {isError && (
            <div className="p-3 rounded-lg bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800/50 text-amber-700 dark:text-amber-300 flex items-center gap-2">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>Could not refresh artifact details: {(error as Error)?.message || 'unknown error'}. Showing list data.</span>
            </div>
          )}

          {/* External Source Banner */}
          <div className="p-3 rounded-lg bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800/80 flex items-center justify-between">
            <div className="space-y-0.5">
              <span className="text-slate-500 dark:text-zinc-500 text-[11px]">Original Reference</span>
              <p className="font-mono text-slate-900 dark:text-zinc-200 font-semibold">{sourceId}</p>
            </div>
            <a
              href={sourceUrl}
              target="_blank"
              rel="noreferrer"
              className="px-3 py-1.5 rounded bg-indigo-50 hover:bg-indigo-100 dark:bg-indigo-600/20 dark:hover:bg-indigo-600/30 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-500/30 font-medium transition flex items-center gap-1.5"
            >
              <span>Open Source URL</span>
              <ExternalLink className="w-3.5 h-3.5" />
            </a>
          </div>

          {/* Tags */}
          {resolved.tags && resolved.tags.length > 0 && (
            <div className="space-y-1.5">
              <span className="text-slate-500 dark:text-zinc-500 font-semibold uppercase text-[10px] tracking-wider flex items-center gap-1">
                <Tag className="w-3 h-3 text-slate-400 dark:text-zinc-400" />
                <span>Tags</span>
              </span>
              <div className="flex flex-wrap gap-1.5">
                {resolved.tags.map((tag, idx) => (
                  <span
                    key={idx}
                    className="px-2 py-0.5 rounded bg-slate-100 dark:bg-zinc-800 text-slate-700 dark:text-zinc-300 border border-slate-200 dark:border-zinc-700 font-mono text-[11px]"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Content Preview */}
          <div className="space-y-1.5">
            <span className="text-slate-500 dark:text-zinc-500 font-semibold uppercase text-[10px] tracking-wider flex items-center gap-1">
              <FileCode className="w-3 h-3 text-slate-400 dark:text-zinc-400" />
              <span>Normalized Engineering Content</span>
            </span>
            <div className="p-4 rounded-lg bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800/80 font-mono text-slate-800 dark:text-zinc-300 whitespace-pre-wrap leading-relaxed max-h-60 overflow-y-auto">
              {textContent}
            </div>
          </div>

          {/* Relationships */}
          {resolved.relationships && resolved.relationships.length > 0 && (
            <div className="space-y-1.5">
              <span className="text-slate-500 dark:text-zinc-500 font-semibold uppercase text-[10px] tracking-wider flex items-center gap-1">
                <Link2 className="w-3 h-3 text-slate-400 dark:text-zinc-400" />
                <span>Relationships ({resolved.relationships.length})</span>
              </span>
              <div className="space-y-1 border border-slate-200 dark:border-zinc-800/80 rounded-lg p-2 bg-slate-50/50 dark:bg-zinc-950/40">
                {resolved.relationships.map((rel, idx) => (
                  <div key={idx} className="flex items-center justify-between text-slate-600 dark:text-zinc-400 font-mono text-[11px] p-1">
                    <span className="truncate mr-2" title={rel.target_id}>Target: {rel.target_id}</span>
                    {onNavigate ? (
                      <button
                        onClick={() => onNavigate(rel.target_id)}
                        className="text-indigo-700 dark:text-indigo-400 bg-indigo-50 dark:bg-indigo-950/40 border border-indigo-200 dark:border-indigo-800/30 px-1.5 py-0.5 rounded hover:bg-indigo-100 dark:hover:bg-indigo-950/70 transition whitespace-nowrap"
                        title="Open related artifact"
                      >
                        {rel.relationship_type} →
                      </button>
                    ) : (
                      <span className="text-indigo-700 dark:text-indigo-400 bg-indigo-50 dark:bg-indigo-950/40 border border-indigo-200 dark:border-indigo-800/30 px-1.5 py-0.5 rounded whitespace-nowrap">
                        {rel.relationship_type}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Raw Metadata */}
          <div className="space-y-1.5">
            <span className="text-slate-500 dark:text-zinc-500 font-semibold uppercase text-[10px] tracking-wider flex items-center gap-1">
              <Layers className="w-3 h-3 text-slate-400 dark:text-zinc-400" />
              <span>Raw Provider Metadata</span>
            </span>
            <pre className="p-3 rounded-lg bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800/80 text-slate-700 dark:text-zinc-400 font-mono text-[10px] overflow-x-auto max-h-40">
              {JSON.stringify(rawMetadata, null, 2)}
            </pre>
          </div>
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-slate-200 dark:border-zinc-800 flex justify-between items-center bg-slate-50 dark:bg-zinc-950/40">
          <span className="text-[11px] text-slate-500 dark:text-zinc-500 font-mono">ID: {resolved.id}</span>
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded bg-slate-100 dark:bg-zinc-800 hover:bg-slate-200 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-300 border border-slate-200 dark:border-zinc-700 font-medium transition"
          >
            Close Viewer
          </button>
        </div>
      </div>
    </div>
  );
};
