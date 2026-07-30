export interface StatusInfo {
  version: string;
  config_path: string;
  db_path: string;
  total_objects: number;
  connectors_count: number;
  db_size_bytes: number;
}

export interface ConnectorInfo {
  id: string;
  provider: 'jira' | 'confluence' | 'github' | 'markdown';
  instance_url: string;
  email: string;
  projects: string[];
  spaces: string[];
  last_synced_at: string | null;
}

export interface SyncProgressInfo {
  is_running: boolean;
  current_connector: string | null;
  fetched: number;
  inserted: number;
  updated: number;
  skipped: number;
  error: string | null;
  last_completed_at: string | null;
}

export interface Relationship {
  target_id: string;
  relationship_type: string;
}

export interface SourceInfo {
  provider: string;
  instance_url: string;
  original_id: string;
  web_url: string;
}

export interface KnowledgeObject {
  id: string;
  object_type: 'ticket' | 'document' | 'specification' | 'design' | 'component';
  title: string;
  summary?: string | null;
  content: string;
  tags: string[];
  relationships: Relationship[];
  source: SourceInfo;
  source_metadata: Record<string, unknown>;
  updated_at: string;
  synced_at: string;
  checksum: string;
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
