import {
  StatusInfo,
  ConnectorInfo,
  SyncProgressInfo,
  KnowledgeObject,
  PaginatedResponse,
  JiraConfigPayload,
  ConfluenceConfigPayload,
  GithubConfigPayload,
  ClickUpConfigPayload,
  MarkdownConfigPayload,
  LocalGitConfigPayload,
} from '../types';

const API_BASE = '/api';

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, options);
  if (!res.ok) {
    const errBody = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(errBody.error || errBody.message || 'API request failed');
  }
  return res.json();
}

export const api = {
  getStatus: () => fetchJson<StatusInfo>(`${API_BASE}/status`),
  
  getConnectors: () => fetchJson<ConnectorInfo[]>(`${API_BASE}/connectors`),
  
  saveJiraConnector: (data: JiraConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/jira`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),
    
  saveConfluenceConnector: (data: ConfluenceConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/confluence`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveGithubConnector: (data: GithubConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/github`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveClickUpConnector: (data: ClickUpConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/clickup`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveMarkdownConnector: (data: MarkdownConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/markdown`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveLocalGitConnector: (data: LocalGitConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/local_git`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

    
  validateCredentials: (data: { provider: string; instance_url: string; email: string; api_token: string }) =>
    fetchJson<{ valid: boolean; message: string }>(`${API_BASE}/connectors/validate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),
    
  triggerSync: (connectorId?: string, full?: boolean) =>
    fetchJson<{ status: string; message: string }>(`${API_BASE}/sync`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ connector_id: connectorId, full }),
    }),
    
  getSyncStatus: () => fetchJson<SyncProgressInfo>(`${API_BASE}/sync/status`),
  
  searchObjects: (query?: string, objectType?: string, tag?: string, limit = 20, page = 1, provider?: string) => {
    const params = new URLSearchParams();
    if (query) params.append('query', query);
    if (objectType) params.append('object_type', objectType);
    if (provider) params.append('provider', provider);
    if (tag) params.append('tag', tag);
    params.append('limit', limit.toString());
    params.append('page', page.toString());
    return fetchJson<PaginatedResponse<KnowledgeObject>>(`${API_BASE}/search?${params.toString()}`);
  },
  
  getObjectById: (id: string) => fetchJson<KnowledgeObject>(`${API_BASE}/objects/${encodeURIComponent(id)}`),
  
  clearData: () =>
    fetchJson<{ success: boolean; message: string }>(`${API_BASE}/storage/clear`, {
      method: 'POST',
    }),

  deleteConnector: (id: string, clearData = true) =>
    fetchJson<{ success: boolean; id: string; cleared_artifacts: number }>(`${API_BASE}/connectors/delete`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id, clear_data: clearData }),
    }),

  selectFolder: () =>
    fetchJson<{ success: boolean; path: string | null }>(`${API_BASE}/dialog/select-folder`, {
      method: 'POST',
    }),
};
