import {
  StatusInfo,
  ConnectorInfo,
  SyncProgressInfo,
  KnowledgeObject,
  PaginatedResponse,
  JiraConfigPayload,
  ConfluenceConfigPayload,
  GithubConfigPayload,
  MarkdownConfigPayload,
  LocalGitConfigPayload,
  ClickupConfigPayload,
  LinearConfigPayload,
  GitlabConfigPayload,
  OpenapiConfigPayload,
  AzureDevopsConfigPayload,
  BitbucketConfigPayload,
  FigmaConfigPayload,
  NotionConfigPayload,
  AsanaConfigPayload,
  SpreadsheetConfigPayload,
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

  saveClickupConnector: (data: ClickupConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/clickup`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveLinearConnector: (data: LinearConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/linear`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveGitlabConnector: (data: GitlabConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/gitlab`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveOpenapiConnector: (data: OpenapiConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/openapi`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveAzureDevopsConnector: (data: AzureDevopsConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/azure_devops`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveBitbucketConnector: (data: BitbucketConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/bitbucket`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveFigmaConnector: (data: FigmaConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/figma`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveNotionConnector: (data: NotionConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/notion`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveAsanaConnector: (data: AsanaConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/asana`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }),

  saveSpreadsheetConnector: (data: SpreadsheetConfigPayload) =>
    fetchJson<{ success: boolean; id: string }>(`${API_BASE}/connectors/spreadsheet`, {
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
