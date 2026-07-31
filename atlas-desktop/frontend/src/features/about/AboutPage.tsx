import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../services/api';
import atlasLogo from '../../assets/logo.svg';
import { Terminal, Cpu, CheckCircle } from 'lucide-react';

export const AboutPage: React.FC = () => {
  const { data: status } = useQuery({ queryKey: ['status'], queryFn: api.getStatus });

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      <div>
        <h2 className="text-xl font-bold text-slate-900 dark:text-zinc-100 tracking-tight">About Atlas</h2>
        <p className="text-xs text-slate-500 dark:text-zinc-400 mt-1">
          Unified Engineering Knowledge Engine (CLI-First Companion).
        </p>
      </div>

      <div className="glass-card p-6 rounded-xl border border-slate-200 dark:border-zinc-800 space-y-5 max-w-xl shadow-xs">
        <div className="flex items-center gap-4 border-b border-slate-200 dark:border-zinc-800 pb-4">
          <div className="w-12 h-12 rounded-xl overflow-hidden border border-slate-200 dark:border-zinc-800 shadow-md shrink-0 bg-slate-900 p-1.5 flex items-center justify-center">
            <img src={atlasLogo} alt="Atlas Logo" className="w-full h-full object-contain" />
          </div>
          <div>
            <h3 className="text-base font-bold text-slate-900 dark:text-zinc-100">Atlas Desktop Companion</h3>
            <p className="text-xs text-slate-500 dark:text-zinc-400">atx CLI v{status?.version || '0.1.0'}</p>
          </div>
        </div>

        <div className="space-y-3 text-xs text-slate-700 dark:text-zinc-300">
          <p className="leading-relaxed">
            Atlas is a <strong>CLI-first Engineering Knowledge Engine</strong> built in Rust. It unifies scattered tickets, documentation, and design specifications into a single local SQLite + FTS5 database.
          </p>

          <div className="bg-slate-50 dark:bg-zinc-950 p-4 rounded-lg border border-slate-200 dark:border-zinc-800 space-y-2 font-mono text-[11px]">
            <div className="text-slate-500 dark:text-zinc-500 uppercase text-[10px] font-semibold">Architectural Guarantees</div>
            <div className="flex items-center gap-2 text-slate-800 dark:text-zinc-300">
              <CheckCircle className="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400" />
              <span>CLI-First & Local-First Design</span>
            </div>
            <div className="flex items-center gap-2 text-slate-800 dark:text-zinc-300">
              <CheckCircle className="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400" />
              <span>Zero Business Logic Duplication in UI</span>
            </div>
            <div className="flex items-center gap-2 text-slate-800 dark:text-zinc-300">
              <CheckCircle className="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400" />
              <span>BM25 FTS5 Full-Text Local Indexing</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
