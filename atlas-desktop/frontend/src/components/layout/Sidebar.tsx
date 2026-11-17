import React from 'react';
import {
  LayoutDashboard,
  Plug,
  BookOpen,
  RefreshCw,
  Settings,
  Info,
  MessageSquareCode,
  Network,
  Clock,
} from 'lucide-react';

import atlasLogo from '../../assets/logo.svg';

interface SidebarProps {
  currentTab: string;
  onTabChange: (tab: string) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentTab,
  onTabChange,
}) => {
  const mainNav = [
    { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { id: 'connectors', label: 'Connectors', icon: Plug },
    { id: 'mcp', label: 'MCP Hub', icon: Network },
    { id: 'knowledge', label: 'Knowledge', icon: BookOpen },
    { id: 'timeline', label: 'Timeline', icon: Clock },
    { id: 'sync', label: 'Sync Status', icon: RefreshCw },
    { id: 'settings', label: 'Settings', icon: Settings },
    { id: 'about', label: 'About', icon: Info },
  ];

  return (
    <aside className="w-14 sm:w-56 shrink-0 border-r border-slate-200 dark:border-zinc-800 bg-white dark:bg-zinc-950 flex flex-col justify-between select-none transition-colors">
      <div>
        {/* Brand Header */}
        <div className="h-14 px-3 sm:px-5 border-b border-slate-200 dark:border-zinc-800 flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-lg overflow-hidden border border-indigo-500/40 shadow-md shadow-indigo-500/20 shrink-0">
            <img src={atlasLogo} alt="Atlas Logo" className="w-full h-full object-cover" />
          </div>
          <div className="hidden sm:block">
            <h2 className="text-sm font-bold text-slate-900 dark:text-zinc-100 tracking-tight">ATLAS</h2>
            <p className="text-[10px] text-slate-400 dark:text-zinc-500 uppercase tracking-widest font-mono font-medium">Companion UI</p>
          </div>
        </div>

        {/* Primary Navigation Items */}
        <nav className="p-1.5 sm:p-3 space-y-1">
          {mainNav.map((item) => {
            const Icon = item.icon;
            const isActive = currentTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => onTabChange(item.id)}
                title={item.label}
                aria-label={item.label}
                className={`w-full flex items-center justify-center sm:justify-start gap-3 px-2 sm:px-3 py-2 rounded-lg text-xs font-medium transition ${
                  isActive
                    ? 'bg-indigo-500/10 dark:bg-indigo-500/15 text-indigo-700 dark:text-indigo-400 shadow-2xs border border-indigo-300/60 dark:border-indigo-500/30 font-semibold'
                    : 'text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200 hover:bg-slate-100 dark:hover:bg-zinc-900'
                }`}
              >
                <Icon className={`w-4 h-4 ${isActive ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-400 dark:text-zinc-500'}`} />
                <span className="hidden sm:inline">{item.label}</span>
              </button>
            );
          })}
        </nav>
      </div>

      <div className="p-1.5 sm:p-3 border-t border-slate-200 dark:border-zinc-800">
        <button
          onClick={() => onTabChange('chat')}
          title="AI Chat"
          aria-label="AI Chat"
          className={`w-full flex items-center justify-center sm:justify-start gap-3 px-2 sm:px-3 py-2 rounded-lg text-xs font-medium transition ${currentTab === 'chat'
            ? 'bg-indigo-500/10 dark:bg-indigo-500/15 text-indigo-700 dark:text-indigo-400 border border-indigo-300/60 dark:border-indigo-500/30 font-semibold'
            : 'text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200 hover:bg-slate-100 dark:hover:bg-zinc-900'}`}
        >
          <MessageSquareCode className="w-4 h-4 text-indigo-600 dark:text-indigo-400" />
          <span className="hidden sm:inline">AI Chat</span>
        </button>
      </div>
    </aside>
  );
};
