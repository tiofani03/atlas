import React, { memo } from 'react';
import { Handle, Position, NodeProps, Node } from '@xyflow/react';
import {
  FileText,
  GitPullRequest,
  GitCommit,
  Layers,
  Sparkles,
  BookOpen,
} from 'lucide-react';
import { GraphNodeData } from '../../types';

export type ArtifactNodeType = Node<GraphNodeData, 'artifact'>;

const getProviderStyle = (provider: string) => {
  const p = provider.toLowerCase();
  switch (p) {
    case 'jira':
      return {
        badge: 'bg-blue-50 dark:bg-blue-950/60 text-blue-700 dark:text-blue-300 border-blue-200 dark:border-blue-800/40',
        border: 'border-blue-300 dark:border-blue-800/60',
        iconColor: 'text-blue-600 dark:text-blue-400',
      };
    case 'github':
      return {
        badge: 'bg-purple-50 dark:bg-purple-950/60 text-purple-700 dark:text-purple-300 border-purple-200 dark:border-purple-800/40',
        border: 'border-purple-300 dark:border-purple-800/60',
        iconColor: 'text-purple-600 dark:text-purple-400',
      };
    case 'confluence':
      return {
        badge: 'bg-cyan-50 dark:bg-cyan-950/60 text-cyan-700 dark:text-cyan-300 border-cyan-200 dark:border-cyan-800/40',
        border: 'border-cyan-300 dark:border-cyan-800/60',
        iconColor: 'text-cyan-600 dark:text-cyan-400',
      };
    case 'markdown':
      return {
        badge: 'bg-emerald-50 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-800/40',
        border: 'border-emerald-300 dark:border-emerald-800/60',
        iconColor: 'text-emerald-600 dark:text-emerald-400',
      };
    case 'figma':
      return {
        badge: 'bg-amber-50 dark:bg-amber-950/60 text-amber-700 dark:text-amber-300 border-amber-200 dark:border-amber-800/40',
        border: 'border-amber-300 dark:border-amber-800/60',
        iconColor: 'text-amber-600 dark:text-amber-400',
      };
    case 'linear':
      return {
        badge: 'bg-violet-50 dark:bg-violet-950/60 text-violet-700 dark:text-violet-300 border-violet-200 dark:border-violet-800/40',
        border: 'border-violet-300 dark:border-violet-800/60',
        iconColor: 'text-violet-600 dark:text-violet-400',
      };
    case 'clickup':
      return {
        badge: 'bg-pink-50 dark:bg-pink-950/60 text-pink-700 dark:text-pink-300 border-pink-200 dark:border-pink-800/40',
        border: 'border-pink-300 dark:border-pink-800/60',
        iconColor: 'text-pink-600 dark:text-pink-400',
      };
    default:
      return {
        badge: 'bg-slate-100 dark:bg-zinc-800 text-slate-700 dark:text-zinc-300 border-slate-200 dark:border-zinc-700',
        border: 'border-slate-300 dark:border-zinc-700',
        iconColor: 'text-slate-600 dark:text-zinc-400',
      };
  }
};

const getKindIcon = (kind: string) => {
  const k = kind.toLowerCase();
  if (k.includes('pull_request') || k.includes('pr')) return GitPullRequest;
  if (k.includes('commit')) return GitCommit;
  if (k.includes('document') || k.includes('spec') || k.includes('markdown')) return BookOpen;
  if (k.includes('ticket') || k.includes('issue') || k.includes('story') || k.includes('epic')) return Layers;
  return FileText;
};

export const ArtifactNode = memo(({ data, selected }: NodeProps<ArtifactNodeType>) => {
  const style = getProviderStyle(data.provider || 'unknown');
  const Icon = getKindIcon(data.kind || '');
  const isRoot = Boolean(data.is_root);

  return (
    <div
      className={`relative w-[260px] rounded-xl transition-all duration-150 p-3.5 bg-white dark:bg-zinc-900 border shadow-xs select-none ${
        style.border
      } ${
        isRoot
          ? 'ring-2 ring-indigo-500 shadow-indigo-500/20 shadow-md'
          : selected
          ? 'ring-2 ring-indigo-400 shadow-sm'
          : 'hover:border-indigo-400/80 hover:shadow-xs'
      }`}
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!w-2.5 !h-2.5 !-left-1.5 !bg-indigo-500 !border-2 !border-white dark:!border-zinc-900 transition hover:!scale-125"
      />

      {/* Header: Provider & Kind */}
      <div className="flex items-center justify-between gap-1.5 mb-1.5">
        <div className="flex items-center gap-1.5 min-w-0">
          <Icon className={`w-3.5 h-3.5 shrink-0 ${style.iconColor}`} />
          <span
            className={`text-[9px] font-mono uppercase font-bold px-1.5 py-0.5 rounded border truncate ${style.badge}`}
          >
            {data.provider}
          </span>
          <span className="text-[10px] text-slate-400 dark:text-zinc-500 font-mono truncate">
            {data.kind}
          </span>
        </div>

        {isRoot && (
          <span className="shrink-0 flex items-center gap-0.5 text-[9px] font-bold px-1.5 py-0.5 rounded bg-indigo-500 text-white shadow-2xs font-mono">
            <Sparkles className="w-2.5 h-2.5" />
            ROOT
          </span>
        )}
      </div>

      {/* Artifact Title */}
      <h4
        className="text-xs font-bold text-slate-900 dark:text-zinc-100 line-clamp-2 leading-tight tracking-tight hover:text-indigo-600 dark:hover:text-indigo-400"
        title={data.title}
      >
        {data.title}
      </h4>

      {/* Footer: Source ID and Repo */}
      <div className="mt-2 pt-2 border-t border-slate-100 dark:border-zinc-800/80 flex items-center justify-between text-[10px] font-mono text-slate-500 dark:text-zinc-400">
        <span className="truncate max-w-[140px] font-semibold text-slate-700 dark:text-zinc-300">
          {data.source_id}
        </span>
        {data.repository && (
          <span className="truncate max-w-[85px] text-slate-400 dark:text-zinc-500">
            {data.repository}
          </span>
        )}
      </div>

      <Handle
        type="source"
        position={Position.Right}
        className="!w-2.5 !h-2.5 !-right-1.5 !bg-indigo-500 !border-2 !border-white dark:!border-zinc-900 transition hover:!scale-125"
      />
    </div>
  );
});

ArtifactNode.displayName = 'ArtifactNode';
