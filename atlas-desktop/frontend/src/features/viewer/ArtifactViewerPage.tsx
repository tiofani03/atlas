import React from 'react';
import { Network, ArrowRight, FileCode, CheckCircle2, Layers } from 'lucide-react';

export const ArtifactViewerPage: React.FC = () => {
  const sampleTrace = [
    { type: 'Epic', title: 'PAY-100: Global Payment Retry Engine', provider: 'Jira' },
    { type: 'Specification', title: 'ADR-042: Exponential Backoff Strategy', provider: 'Confluence' },
    { type: 'Story', title: 'PAY-102: Implement Stripe Webhook Handler', provider: 'Jira' },
    { type: 'Task', title: 'PAY-108: Add Database Migration for Retries', provider: 'Jira' },
    { type: 'PR', title: 'PR #402: Add retry queue migration', provider: 'GitHub (Future)' },
  ];

  return (
    <div className="p-6 space-y-6">
      <div>
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-bold text-zinc-100 tracking-tight">Artifact Relationship Visualizer</h2>
          <span className="text-[10px] px-2 py-0.5 rounded bg-indigo-950/60 border border-indigo-800/40 text-indigo-300 font-mono">
            Feature Flag Enabled
          </span>
        </div>
        <p className="text-xs text-zinc-400 mt-1">
          Read-only visualizer tracking knowledge lineage: Epic → Specification → Story → Task → PR → Release.
        </p>
      </div>

      <div className="glass-panel p-6 rounded-xl border border-zinc-800 space-y-6">
        <h3 className="text-sm font-bold text-zinc-200 flex items-center gap-2">
          <Network className="w-4 h-4 text-indigo-400" />
          <span>Knowledge Dependency Trace</span>
        </h3>

        <div className="flex flex-col md:flex-row items-center gap-3 overflow-x-auto p-4 bg-zinc-950/80 rounded-xl border border-zinc-800/80">
          {sampleTrace.map((item, idx) => (
            <React.Fragment key={idx}>
              <div className="glass-card p-4 rounded-xl space-y-1.5 min-w-[200px] border border-zinc-800 shrink-0">
                <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-indigo-950/60 border border-indigo-800/40 text-indigo-300 font-bold">
                  {item.type}
                </span>
                <h4 className="text-xs font-bold text-zinc-100 mt-1 truncate">{item.title}</h4>
                <p className="text-[10px] text-zinc-500 font-mono">{item.provider}</p>
              </div>
              {idx < sampleTrace.length - 1 && (
                <ArrowRight className="w-4 h-4 text-zinc-600 shrink-0 hidden md:block" />
              )}
            </React.Fragment>
          ))}
        </div>
      </div>
    </div>
  );
};
