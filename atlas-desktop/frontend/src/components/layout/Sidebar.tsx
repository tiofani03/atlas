import React from 'react';
import {
  LayoutDashboard,
  Plug,
  BookOpen,
  RefreshCw,
  Settings,
  Info,
  Box,
  MessageSquareCode,
  Network,
} from 'lucide-react';

import atlasLogo from '../../assets/logo.jpg';

interface SidebarProps {
  currentTab: string;
  onTabChange: (tab: string) => void;
  featureFlags: {
    aiChat: boolean;
    artifactViewer: boolean;
  };
  onToggleFeature: (feature: 'aiChat' | 'artifactViewer') => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentTab,
  onTabChange,
  featureFlags,
  onToggleFeature,
}) => {
  const mainNav = [
    { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { id: 'connectors', label: 'Connectors', icon: Plug },
    { id: 'knowledge', label: 'Knowledge', icon: BookOpen },
    { id: 'sync', label: 'Sync Status', icon: RefreshCw },
    { id: 'settings', label: 'Settings', icon: Settings },
    { id: 'about', label: 'About', icon: Info },
  ];

  return (
    <aside className="w-56 border-r border-slate-200 dark:border-zinc-800 bg-white dark:bg-zinc-950 flex flex-col justify-between select-none transition-colors">
      <div>
        {/* Brand Header */}
        <div className="h-14 px-5 border-b border-slate-200 dark:border-zinc-800 flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-lg overflow-hidden border border-indigo-500/40 shadow-md shadow-indigo-500/20 shrink-0">
            <img src={atlasLogo} alt="Atlas Logo" className="w-full h-full object-cover" />
          </div>
          <div>
            <h2 className="text-sm font-bold text-slate-900 dark:text-zinc-100 tracking-tight">ATLAS</h2>
            <p className="text-[10px] text-slate-400 dark:text-zinc-500 uppercase tracking-widest font-mono font-medium">Companion UI</p>
          </div>
        </div>

        {/* Primary Navigation Items */}
        <nav className="p-3 space-y-1">
          {mainNav.map((item) => {
            const Icon = item.icon;
            const isActive = currentTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => onTabChange(item.id)}
                className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-xs font-medium transition ${
                  isActive
                    ? 'bg-indigo-500/10 dark:bg-indigo-500/15 text-indigo-700 dark:text-indigo-400 shadow-2xs border border-indigo-300/60 dark:border-indigo-500/30 font-semibold'
                    : 'text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200 hover:bg-slate-100 dark:hover:bg-zinc-900'
                }`}
              >
                <Icon className={`w-4 h-4 ${isActive ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-400 dark:text-zinc-500'}`} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Feature Flags / Experimental Section */}
      <div className="p-3 border-t border-slate-200 dark:border-zinc-800/80 bg-slate-50/50 dark:bg-zinc-950/60">
        <div className="text-[10px] uppercase font-mono text-slate-400 dark:text-zinc-500 font-semibold px-3 mb-2 tracking-wider">
          Experimental Flags
        </div>
        <div className="space-y-1">
          <button
            onClick={() => {
              onToggleFeature('aiChat');
              if (!featureFlags.aiChat) onTabChange('chat');
            }}
            className={`w-full flex items-center justify-between px-3 py-1.5 rounded-lg text-xs transition ${
              featureFlags.aiChat && currentTab === 'chat'
                ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/40'
                : 'text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-300 hover:bg-slate-100 dark:hover:bg-zinc-900'
            }`}
          >
            <div className="flex items-center gap-2">
              <MessageSquareCode className="w-3.5 h-3.5 text-indigo-600 dark:text-indigo-400" />
              <span>AI Chat</span>
            </div>
            <span
              className={`text-[9px] px-1.5 py-0.2 rounded font-mono ${
                featureFlags.aiChat ? 'bg-indigo-100 dark:bg-indigo-500/20 text-indigo-700 dark:text-indigo-300' : 'bg-slate-200 dark:bg-zinc-800 text-slate-500 dark:text-zinc-500'
              }`}
            >
              {featureFlags.aiChat ? 'ON' : 'OFF'}
            </span>
          </button>

          <button
            onClick={() => {
              onToggleFeature('artifactViewer');
              if (!featureFlags.artifactViewer) onTabChange('viewer');
            }}
            className={`w-full flex items-center justify-between px-3 py-1.5 rounded-lg text-xs transition ${
              featureFlags.artifactViewer && currentTab === 'viewer'
                ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/40'
                : 'text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-300 hover:bg-slate-100 dark:hover:bg-zinc-900'
            }`}
          >
            <div className="flex items-center gap-2">
              <Network className="w-3.5 h-3.5 text-indigo-600 dark:text-indigo-400" />
              <span>Artifacts</span>
            </div>
            <span
              className={`text-[9px] px-1.5 py-0.2 rounded font-mono ${
                featureFlags.artifactViewer ? 'bg-indigo-100 dark:bg-indigo-500/20 text-indigo-700 dark:text-indigo-300' : 'bg-slate-200 dark:bg-zinc-800 text-slate-500 dark:text-zinc-500'
              }`}
            >
              {featureFlags.artifactViewer ? 'ON' : 'OFF'}
            </span>
          </button>
        </div>
      </div>
    </aside>
  );
};
