export interface StatusInfo {
  version: string;
  config_path: string;
  db_path: string;
  total_artifacts?: number;
  total_objects?: number;
  connectors_count: number;
  db_size_bytes: number;
}

export interface ConnectorInfo {
  id: string;
  provider: 'jira' | 'confluence' | 'github' | 'linear' | 'asana' | 'slack' | 'openapi' | 'figma' | 'azure_devops' | 'markdown' | 'local_git' | 'notion' | 'gitlab' | 'bitbucket';
  instance_url: string;
  email: string;
  projects: string[];
  spaces: string[];
  repos: string[];
  path?: string;
  paths?: string[];
  glob_patterns?: string[];
  last_synced_at: string | null;
}

export interface SyncProgressInfo {
  is_running: boolean;
  current_connector: string | null;
  phase?: string | null;
  current?: number;
  total?: number;
  percentage?: number;
  fetched: number;
  inserted: number;
  updated: number;
  skipped: number;
  error: string | null;
  last_completed_at: string | null;
}

export interface ArtifactRelationship {
  source_id: string;
  target_id: string;
  relationship_type: string;
}

export interface SourceInfo {
  provider: string;
  instance_url: string;
  original_id: string;
  web_url: string;
}

export interface KnowledgeArtifact {
  id: string;
  kind?: string;
  object_type?: string;
  title: string;
  summary?: string | null;
  body?: string;
  content?: string;
  provider?: string;
  source_id?: string;
  source_url?: string;
  repository?: string | null;
  tags: string[];
  relationships: ArtifactRelationship[];
  source?: SourceInfo;
  source_metadata?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  created_at?: string | null;
  updated_at: string;
  synced_at: string;
  checksum: string;
}

export type KnowledgeObject = KnowledgeArtifact;

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  limit: number;
  total_pages: number;
}

export interface JiraConfigPayload {
  id: string;
  instance_url: string;
  email: string;
  api_token?: string;
  projects?: string[];
}

export interface ConfluenceConfigPayload {
  id: string;
  instance_url: string;
  email: string;
  api_token?: string;
  spaces?: string[];
}

export interface GithubConfigPayload {
  id: string;
  instance_url?: string;
  api_token?: string;
  repos?: string[];
}

export interface MarkdownConfigPayload {
  id: string;
  path?: string;
  paths?: string[];
  glob_patterns?: string[];
}

export interface LocalGitConfigPayload {
  id: string;
  path?: string;
  paths?: string[];
}


