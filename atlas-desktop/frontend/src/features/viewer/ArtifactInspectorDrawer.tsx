import React, { useState } from 'react';
import {
  X,
  Target,
  Maximize2,
  ExternalLink,
  Tag,
  GitFork,
  FileText,
  Calendar,
  Sparkles,
  ChevronDown,
  ChevronRight,
  Loader2,
  Copy,
  Check,
} from 'lucide-react';
import { GraphNodeData } from '../../types';

interface ArtifactInspectorDrawerProps {
  node: GraphNodeData | null;
  isOpen: boolean;
  onClose: () => void;
  onFocusAsRoot: (id: string) => void;
  onExpandNeighbors: (id: string) => void;
  isExpanding?: boolean;
}

export const ArtifactInspectorDrawer: React.FC<ArtifactInspectorDrawerProps> = ({
  node,
  isOpen,
  onClose,
  onFocusAsRoot,
  onExpandNeighbors,
  isExpanding = false,
}) => {
  const [showRawMeta, setShowRawMeta] = useState(false);
  const [copied, setCopied] = useState(false);

  if (!isOpen || !node) return null;

  const handleCopyId = () => {
    navigator.clipboard.writeText(node.id);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <aside className="absolute right-0 top-0 bottom-0 w-80 md:w-96 bg-white/95 dark:bg-zinc-900/95 backdrop-blur-md border-l border-slate-200 dark:border-zinc-800 shadow-2xl z-20 flex flex-col transition-all duration-200 animate-in slide-in-from-right">
      {/* Drawer Header */}
      <div className="p-4 border-b border-slate-200 dark:border-zinc-800 flex items-center justify-between">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-[10px] uppercase font-mono font-bold px-2 py-0.5 rounded bg-indigo-50 dark:bg-indigo-950/60 border border-indigo-200 dark:border-indigo-800/40 text-indigo-700 dark:text-indigo-300">
            {node.provider}
          </span>
          <span className="text-xs font-mono text-slate-500 dark:text-zinc-400 truncate">
            {node.kind}
          </span>
          {node.is_root && (
            <span className="flex items-center gap-1 text-[9px] font-bold px-1.5 py-0.5 rounded bg-amber-500/15 border border-amber-500/30 text-amber-600 dark:text-amber-400 font-mono">
              <Sparkles className="w-2.5 h-2.5" />
              Focal
            </span>
          )}
        </div>

        <button
          onClick={onClose}
          className="p-1 rounded-md text-slate-400 hover:text-slate-600 dark:hover:text-zinc-200 hover:bg-slate-100 dark:hover:bg-zinc-800 transition"
          aria-label="Close Inspector"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Drawer Body */}
      <div className="flex-1 overflow-y-auto p-4 space-y-5">
        {/* Title and ID */}
        <div>
          <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100 leading-snug">
            {node.title}
          </h3>
          <div className="flex items-center gap-1.5 mt-2">
            <span className="text-xs font-mono text-slate-500 dark:text-zinc-400 truncate">
              {node.source_id}
            </span>
            <button
              onClick={handleCopyId}
              className="text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 p-0.5 transition"
              title="Copy ID"
            >
              {copied ? <Check className="w-3.5 h-3.5 text-emerald-500" /> : <Copy className="w-3.5 h-3.5" />}
            </button>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="grid grid-cols-2 gap-2">
          <button
            onClick={() => onFocusAsRoot(node.id)}
            disabled={node.is_root}
            className={`flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold border transition ${
              node.is_root
                ? 'bg-slate-100 dark:bg-zinc-800 text-slate-400 dark:text-zinc-500 border-slate-200 dark:border-zinc-800 cursor-not-allowed'
                : 'bg-indigo-600 hover:bg-indigo-500 text-white border-transparent shadow-xs hover:shadow-sm'
            }`}
          >
            <Target className="w-3.5 h-3.5" />
            <span>Focus Root</span>
          </button>

          <button
            onClick={() => onExpandNeighbors(node.id)}
            disabled={isExpanding}
            className="flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold bg-slate-100 dark:bg-zinc-800 hover:bg-slate-200 dark:hover:bg-zinc-700 text-slate-700 dark:text-zinc-200 border border-slate-200 dark:border-zinc-700 transition"
          >
            {isExpanding ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <Maximize2 className="w-3.5 h-3.5" />
            )}
            <span>Expand (+1)</span>
          </button>
        </div>

        {/* Repository & External Link */}
        <div className="p-3 rounded-lg bg-slate-50 dark:bg-zinc-950/60 border border-slate-200 dark:border-zinc-800/80 space-y-2 text-xs">
          {node.repository && (
            <div className="flex items-center justify-between">
              <span className="text-slate-500 dark:text-zinc-400 flex items-center gap-1.5 font-mono">
                <GitFork className="w-3.5 h-3.5 text-slate-400" />
                Repository
              </span>
              <span className="font-mono text-slate-700 dark:text-zinc-300 font-medium">
                {node.repository}
              </span>
            </div>
          )}

          {node.source_url && (
            <div className="flex items-center justify-between pt-1">
              <span className="text-slate-500 dark:text-zinc-400 flex items-center gap-1.5 font-mono">
                <ExternalLink className="w-3.5 h-3.5 text-slate-400" />
                Source
              </span>
              <a
                href={node.source_url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-indigo-600 dark:text-indigo-400 hover:underline font-mono text-[11px] flex items-center gap-1 font-medium"
              >
                Open in {node.provider}
              </a>
            </div>
          )}
        </div>

        {/* Summary Description */}
        {node.summary ? (
          <div className="space-y-1.5">
            <h4 className="text-xs font-semibold text-slate-600 dark:text-zinc-400 uppercase tracking-wider font-mono">
              Summary
            </h4>
            <div className="p-3 rounded-lg bg-slate-50 dark:bg-zinc-950/60 border border-slate-200 dark:border-zinc-800/80 text-xs text-slate-700 dark:text-zinc-300 leading-relaxed max-h-48 overflow-y-auto whitespace-pre-wrap">
              {node.summary}
            </div>
          </div>
        ) : (
          <div className="text-xs text-slate-400 dark:text-zinc-500 italic">
            No summary cached for this artifact.
          </div>
        )}

        {/* Tags */}
        {node.tags && node.tags.length > 0 && (
          <div className="space-y-1.5">
            <h4 className="text-xs font-semibold text-slate-600 dark:text-zinc-400 uppercase tracking-wider font-mono flex items-center gap-1.5">
              <Tag className="w-3 h-3 text-slate-400" />
              Tags
            </h4>
            <div className="flex flex-wrap gap-1.5">
              {node.tags.map((tag, idx) => (
                <span
                  key={idx}
                  className="text-[10px] px-2 py-0.5 rounded-md bg-slate-100 dark:bg-zinc-800 text-slate-600 dark:text-zinc-300 font-mono border border-slate-200 dark:border-zinc-700"
                >
                  {tag}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Collapsible Metadata */}
        {node.metadata && Object.keys(node.metadata).length > 0 && (
          <div className="border-t border-slate-200 dark:border-zinc-800 pt-3">
            <button
              onClick={() => setShowRawMeta((prev) => !prev)}
              className="w-full flex items-center justify-between text-xs text-slate-500 dark:text-zinc-400 hover:text-slate-800 dark:hover:text-zinc-200 font-mono py-1"
            >
              <span>Raw Metadata ({Object.keys(node.metadata).length} keys)</span>
              {showRawMeta ? (
                <ChevronDown className="w-3.5 h-3.5" />
              ) : (
                <ChevronRight className="w-3.5 h-3.5" />
              )}
            </button>
            {showRawMeta && (
              <pre className="mt-2 p-2.5 rounded-lg bg-slate-100 dark:bg-zinc-950 text-[10px] font-mono text-slate-800 dark:text-zinc-200 overflow-x-auto max-h-52 border border-slate-200 dark:border-zinc-800">
                {JSON.stringify(node.metadata, null, 2)}
              </pre>
            )}
          </div>
        )}
      </div>
    </aside>
  );
};
