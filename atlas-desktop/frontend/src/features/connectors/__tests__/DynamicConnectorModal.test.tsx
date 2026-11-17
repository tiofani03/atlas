import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { DynamicConnectorModal } from '../DynamicConnectorModal';
import type { ConnectorManifest } from '../../../types';
import { api } from '../../../services/api';

vi.mock('../../../services/api', () => ({
  api: {
    saveConnectorGeneric: vi.fn(),
    validateCredentials: vi.fn(),
    selectFolder: vi.fn(),
  },
}));

const githubManifest: ConnectorManifest = {
  id: 'github',
  provider: 'github',
  version: '2.0.0',
  name: 'GitHub',
  description: 'Sync repositories, issues, and PRs.',
  config_fields: [
    {
      key: 'api_token',
      label: 'Personal Access Token',
      field_type: 'password',
      required: true,
      placeholder: '${GITHUB_TOKEN}',
      help_text: 'GitHub PAT',
    },
    {
      key: 'repos',
      label: 'Repositories',
      field_type: 'list',
      required: true,
      placeholder: 'owner/repo1, owner/repo2',
      help_text: 'Comma-separated',
    },
  ],
};

describe('DynamicConnectorModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders fields from manifest and submits parsed list payload', async () => {
    const onClose = vi.fn();
    const onSuccess = vi.fn();
    vi.mocked(api.saveConnectorGeneric).mockResolvedValue({ success: true, id: 'github-main' });

    render(
      <DynamicConnectorModal
        isOpen={true}
        manifest={githubManifest}
        onClose={onClose}
        onSuccess={onSuccess}
      />
    );

    expect(screen.getByText('Configure GitHub')).toBeInTheDocument();
    const tokenInput = screen.getByPlaceholderText('${GITHUB_TOKEN}');
    const reposInput = screen.getByPlaceholderText('owner/repo1, owner/repo2');

    fireEvent.change(tokenInput, { target: { value: 'ghp_test_123' } });
    fireEvent.change(reposInput, { target: { value: 'acme/api, acme/web' } });

    fireEvent.click(screen.getByRole('button', { name: /save connector/i }));

    await waitFor(() => {
      expect(api.saveConnectorGeneric).toHaveBeenCalledWith('github', {
        id: 'github-main',
        api_token: 'ghp_test_123',
        repos: ['acme/api', 'acme/web'],
      });
    });
    expect(onSuccess).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it('runs credential validation when Test Connection is clicked', async () => {
    vi.mocked(api.validateCredentials).mockResolvedValue({
      valid: true,
      message: 'Connected successfully',
    });

    render(
      <DynamicConnectorModal
        isOpen={true}
        manifest={githubManifest}
        onClose={vi.fn()}
        onSuccess={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /test connection/i }));

    await waitFor(() => {
      expect(screen.getByText('Connected successfully')).toBeInTheDocument();
    });
  });
});
