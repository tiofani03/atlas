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
    <aside className="w-56 border-r border-zinc-800 bg-zinc-950 flex flex-col justify-between select-none">
      <div>
        {/* Brand Header */}
        <div className="h-14 px-5 border-b border-zinc-800 flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-lg bg-gradient-to-tr from-indigo-600 to-indigo-400 flex items-center justify-center text-white shadow-md shadow-indigo-600/30">
            <Box className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-sm font-bold text-zinc-100 tracking-tight">ATLAS</h2>
            <p className="text-[10px] text-zinc-500 uppercase tracking-widest font-mono">Companion UI</p>
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
                className={`w-full flex items-center gap-3 px-3 py-2 rounded-md text-xs font-medium transition ${
                  isActive
                    ? 'bg-zinc-800 text-zinc-100 shadow-sm border border-zinc-700/50'
                    : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900'
                }`}
              >
                <Icon className={`w-4 h-4 ${isActive ? 'text-indigo-400' : 'text-zinc-400'}`} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Feature Flags / Experimental Section */}
      <div className="p-3 border-t border-zinc-800/80 bg-zinc-950/60">
        <div className="text-[10px] uppercase font-mono text-zinc-500 font-semibold px-3 mb-2 tracking-wider">
          Experimental Flags
        </div>
        <div className="space-y-1">
          <button
            onClick={() => {
              onToggleFeature('aiChat');
              if (!featureFlags.aiChat) onTabChange('chat');
            }}
            className={`w-full flex items-center justify-between px-3 py-1.5 rounded-md text-xs transition ${
              featureFlags.aiChat && currentTab === 'chat'
                ? 'bg-indigo-950/50 text-indigo-300 border border-indigo-800/40'
                : 'text-zinc-400 hover:text-zinc-300 hover:bg-zinc-900'
            }`}
          >
            <div className="flex items-center gap-2">
              <MessageSquareCode className="w-3.5 h-3.5 text-indigo-400" />
              <span>AI Chat</span>
            </div>
            <span
              className={`text-[9px] px-1.5 py-0.2 rounded font-mono ${
                featureFlags.aiChat ? 'bg-indigo-500/20 text-indigo-300' : 'bg-zinc-800 text-zinc-500'
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
            className={`w-full flex items-center justify-between px-3 py-1.5 rounded-md text-xs transition ${
              featureFlags.artifactViewer && currentTab === 'viewer'
                ? 'bg-indigo-950/50 text-indigo-300 border border-indigo-800/40'
                : 'text-zinc-400 hover:text-zinc-300 hover:bg-zinc-900'
            }`}
          >
            <div className="flex items-center gap-2">
              <Network className="w-3.5 h-3.5 text-indigo-400" />
              <span>Artifacts</span>
            </div>
            <span
              className={`text-[9px] px-1.5 py-0.2 rounded font-mono ${
                featureFlags.artifactViewer ? 'bg-indigo-500/20 text-indigo-300' : 'bg-zinc-800 text-zinc-500'
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
