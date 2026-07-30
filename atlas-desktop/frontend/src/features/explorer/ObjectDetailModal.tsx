import React from 'react';
import { X, ExternalLink, Tag, Link2, FileCode, Layers } from 'lucide-react';
import { KnowledgeObject } from '../../types';

interface ModalProps {
  object: KnowledgeObject | null;
  onClose: () => void;
}

export const ObjectDetailModal: React.FC<ModalProps> = ({ object, onClose }) => {
  if (!object) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl w-full max-w-3xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="p-5 border-b border-zinc-800 flex items-start justify-between bg-zinc-950/40">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-indigo-950/60 border border-indigo-800/40 text-indigo-300 font-bold">
                {object.object_type}
              </span>
              <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-zinc-800 text-zinc-400">
                {object.source.provider}
              </span>
            </div>
            <h3 className="text-base font-bold text-zinc-100">{object.title}</h3>
            {object.summary && <p className="text-xs text-zinc-400">{object.summary}</p>}
          </div>

          <button onClick={onClose} className="text-zinc-400 hover:text-zinc-200">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content Body */}
        <div className="p-6 overflow-y-auto space-y-5 text-xs">
          {/* External Source Banner */}
          <div className="p-3 rounded-lg bg-zinc-950 border border-zinc-800/80 flex items-center justify-between">
            <div className="space-y-0.5">
              <span className="text-zinc-500 text-[11px]">Original Reference</span>
              <p className="font-mono text-zinc-200">{object.source.original_id}</p>
            </div>
            <a
              href={object.source.web_url}
              target="_blank"
              rel="noreferrer"
              className="px-3 py-1.5 rounded bg-indigo-600/20 hover:bg-indigo-600/30 text-indigo-300 border border-indigo-500/30 font-medium transition flex items-center gap-1.5"
            >
              <span>Open Source URL</span>
              <ExternalLink className="w-3.5 h-3.5" />
            </a>
          </div>

          {/* Tags */}
          {object.tags.length > 0 && (
            <div className="space-y-1.5">
              <span className="text-zinc-500 font-semibold uppercase text-[10px] tracking-wider flex items-center gap-1">
                <Tag className="w-3 h-3 text-zinc-400" />
                <span>Tags</span>
              </span>
              <div className="flex flex-wrap gap-1.5">
                {object.tags.map((tag, idx) => (
                  <span
                    key={idx}
                    className="px-2 py-0.5 rounded bg-zinc-800 text-zinc-300 font-mono text-[11px]"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Content Preview */}
          <div className="space-y-1.5">
            <span className="text-zinc-500 font-semibold uppercase text-[10px] tracking-wider flex items-center gap-1">
              <FileCode className="w-3 h-3 text-zinc-400" />
              <span>Normalized Knowledge Content</span>
            </span>
            <div className="p-4 rounded-lg bg-zinc-950 border border-zinc-800/80 font-mono text-zinc-300 whitespace-pre-wrap leading-relaxed max-h-60 overflow-y-auto">
              {object.content || 'No text content extracted.'}
            </div>
          </div>

          {/* Relationships */}
          {object.relationships.length > 0 && (
            <div className="space-y-1.5">
              <span className="text-zinc-500 font-semibold uppercase text-[10px] tracking-wider flex items-center gap-1">
                <Link2 className="w-3 h-3 text-zinc-400" />
                <span>Relationships ({object.relationships.length})</span>
              </span>
              <div className="space-y-1 border border-zinc-800/80 rounded-lg p-2 bg-zinc-950/40">
                {object.relationships.map((rel, idx) => (
                  <div key={idx} className="flex items-center justify-between text-zinc-400 font-mono text-[11px] p-1">
                    <span>Target: {rel.target_id.slice(0, 16)}...</span>
                    <span className="text-indigo-400 bg-indigo-950/40 px-1.5 py-0.5 rounded">
                      {rel.relationship_type}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Source Metadata JSON */}
          <div className="space-y-1.5">
            <span className="text-zinc-500 font-semibold uppercase text-[10px] tracking-wider flex items-center gap-1">
              <Layers className="w-3 h-3 text-zinc-400" />
              <span>Raw Provider Metadata</span>
            </span>
            <pre className="p-3 rounded-lg bg-zinc-950 border border-zinc-800/80 text-zinc-400 font-mono text-[10px] overflow-x-auto max-h-40">
              {JSON.stringify(object.source_metadata, null, 2)}
            </pre>
          </div>
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-zinc-800 flex justify-between items-center bg-zinc-950/40">
          <span className="text-[11px] text-zinc-500 font-mono">ID: {object.id}</span>
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 font-medium transition"
          >
            Close Viewer
          </button>
        </div>
      </div>
    </div>
  );
};
