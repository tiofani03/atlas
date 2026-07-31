import React, { useState, useEffect } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Header } from './components/layout/Header';
import { Sidebar } from './components/layout/Sidebar';
import { DashboardPage } from './features/dashboard/DashboardPage';
import { ConnectorsPage } from './features/connectors/ConnectorsPage';
import { ExplorerPage } from './features/explorer/ExplorerPage';
import { SyncPage } from './features/sync/SyncPage';
import { SettingsPage } from './features/settings/SettingsPage';
import { AboutPage } from './features/about/AboutPage';
import { ChatPage } from './features/chat/ChatPage';
import { ArtifactViewerPage } from './features/viewer/ArtifactViewerPage';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      staleTime: 5000,
    },
  },
});

export const App: React.FC = () => {
  const [currentTab, setCurrentTab] = useState('dashboard');
  const [isDarkMode, setIsDarkMode] = useState(() => {
    return localStorage.getItem('theme') !== 'light';
  });

  const [featureFlags, setFeatureFlags] = useState({
    aiChat: false,
    artifactViewer: false,
  });

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.remove('light');
      document.documentElement.classList.add('dark');
      localStorage.setItem('theme', 'dark');
    } else {
      document.documentElement.classList.remove('dark');
      document.documentElement.classList.add('light');
      localStorage.setItem('theme', 'light');
    }
  }, [isDarkMode]);

  const handleToggleTheme = () => {
    setIsDarkMode((prev) => !prev);
  };

  const handleToggleFeature = (feature: 'aiChat' | 'artifactViewer') => {
    setFeatureFlags((prev) => ({
      ...prev,
      [feature]: !prev[feature],
    }));
  };

  const renderContent = () => {
    switch (currentTab) {
      case 'dashboard':
        return <DashboardPage />;
      case 'connectors':
        return <ConnectorsPage />;
      case 'knowledge':
        return <ExplorerPage />;
      case 'sync':
        return <SyncPage />;
      case 'settings':
        return <SettingsPage />;
      case 'about':
        return <AboutPage />;
      case 'chat':
        return featureFlags.aiChat ? <ChatPage /> : <DashboardPage />;
      case 'viewer':
        return featureFlags.artifactViewer ? <ArtifactViewerPage /> : <DashboardPage />;
      default:
        return <DashboardPage />;
    }
  };

  return (
    <QueryClientProvider client={queryClient}>
      <div className="flex h-screen w-screen overflow-hidden bg-slate-50 dark:bg-zinc-950 text-slate-900 dark:text-zinc-100 transition-colors duration-200">
        <Sidebar
          currentTab={currentTab}
          onTabChange={setCurrentTab}
          featureFlags={featureFlags}
          onToggleFeature={handleToggleFeature}
        />
        <div className="flex-1 flex flex-col min-w-0 overflow-y-auto">
          <Header
            currentTab={currentTab}
            isDarkMode={isDarkMode}
            onToggleTheme={handleToggleTheme}
          />
          <main className="flex-1 bg-slate-100/60 dark:bg-zinc-950/90 transition-colors duration-200">
            {renderContent()}
          </main>
        </div>
      </div>
    </QueryClientProvider>
  );
};

export default App;
