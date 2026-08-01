import React, { useState, useRef } from 'react';
import { X, Loader2, Plus, Trash2, Folder, FolderOpen } from 'lucide-react';
import { api } from '../../services/api';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  initialConfig?: {
    id: string;
    path?: string;
    paths?: string[];
    glob_patterns?: string[];
  };
}

export const ConfigureMarkdownModal: React.FC<ModalProps> = ({ isOpen, onClose, onSuccess, initialConfig }) => {
  const [id, setId] = useState(initialConfig?.id || 'markdown-docs');

  // Initialize paths from initialConfig.paths or split initialConfig.path by comma
  const initialPaths = initialConfig?.paths && initialConfig.paths.length > 0
    ? initialConfig.paths
    : initialConfig?.path
    ? initialConfig.path.split(',').map((p) => p.trim()).filter(Boolean)
    : ['./docs'];

  const [paths, setPaths] = useState<string[]>(initialPaths);
  const [globPatterns, setGlobPatterns] = useState(initialConfig?.glob_patterns?.join(', ') || '*.md, *.markdown, *.mdx');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isPickerOpening, setIsPickerOpening] = useState(false);

  const fileInputRefs = useRef<(HTMLInputElement | null)[]>([]);

  if (!isOpen) return null;

  const handleAddPath = () => {
    setPaths([...paths, '']);
  };

  const handleRemovePath = (index: number) => {
    if (paths.length === 1) return;
    setPaths(paths.filter((_, i) => i !== index));
  };

  const handlePathChange = (index: number, value: string) => {
    const updated = [...paths];
    updated[index] = value;
    setPaths(updated);
  };

  const handleBrowseFolder = async (index: number) => {
    if (isPickerOpening) return;
    setIsPickerOpening(true);

    try {
      const res = await api.selectFolder();
      if (res.success && res.path) {
        handlePathChange(index, res.path);
        setIsPickerOpening(false);
        return;
      }
    } catch {
      // Ignore error and fall back to HTML5 file input
    }

    // Fallback to HTML5 directory input
    const inputEl = fileInputRefs.current[index];
    if (inputEl) {
      inputEl.value = '';
      inputEl.click();
    }
    setTimeout(() => setIsPickerOpening(false), 1000);
  };

  const handleFolderSelected = (index: number, e: React.ChangeEvent<HTMLInputElement>) => {
    setIsPickerOpening(false);
    if (e.target.files && e.target.files.length > 0) {
      const firstFile = e.target.files[0] as unknown as { path?: string; webkitRelativePath?: string; name: string };
      let folderPath = '';
      if (firstFile.path) {
        // Electron / Node environment absolute path
        const lastSlash = Math.max(firstFile.path.lastIndexOf('/'), firstFile.path.lastIndexOf('\\'));
        folderPath = lastSlash !== -1 ? firstFile.path.substring(0, lastSlash) : firstFile.path;
      } else if (firstFile.webkitRelativePath) {
        folderPath = `./${firstFile.webkitRelativePath.split('/')[0]}`;
      } else if (firstFile.name) {
        folderPath = `./${firstFile.name}`;
      }

      if (folderPath) {
        handlePathChange(index, folderPath);
      }
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    setErrorMessage(null);
    try {
      const validPaths = paths.map((p) => p.trim()).filter(Boolean);
      if (validPaths.length === 0) {
        setErrorMessage('Please specify at least one directory path.');
        setIsSubmitting(false);
        return;
      }

      const globArray = globPatterns.split(',').map((g) => g.trim()).filter(Boolean);
      await api.saveMarkdownConnector({
        id,
        path: validPaths.join(', '),
        paths: validPaths,
        glob_patterns: globArray.length > 0 ? globArray : undefined,
      });
      onSuccess();
      onClose();
    } catch (err: unknown) {
      setErrorMessage((err as Error).message || 'Failed to save Markdown connector');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-900/40 dark:bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-xl w-full max-w-md p-6 space-y-4 shadow-2xl">
        <div className="flex items-center justify-between border-b border-slate-200 dark:border-zinc-800 pb-3">
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 rounded bg-emerald-50 dark:bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 flex items-center justify-center font-bold text-xs border border-emerald-200 dark:border-emerald-500/30">
              MD
            </div>
            <h3 className="text-sm font-bold text-slate-900 dark:text-zinc-100">Configure Markdown Connector</h3>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 dark:text-zinc-400 dark:hover:text-zinc-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-3.5 text-xs">
          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">Connector ID</label>
            <input
              type="text"
              value={id}
              onChange={(e) => setId(e.target.value)}
              required
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-emerald-500 font-mono"
            />
          </div>

          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="block text-slate-600 dark:text-zinc-400 font-medium">
                Target Directories ({paths.length})
              </label>
              <button
                type="button"
                onClick={handleAddPath}
                className="text-[11px] font-medium text-emerald-600 dark:text-emerald-400 hover:text-emerald-500 flex items-center gap-1"
              >
                <Plus className="w-3 h-3" />
                <span>Add Path</span>
              </button>
            </div>

            <div className="space-y-2 max-h-48 overflow-y-auto pr-1">
              {paths.map((p, idx) => (
                <div key={idx} className="flex items-center gap-1.5">
                  <div className="relative flex-1">
                    <Folder className="w-3.5 h-3.5 text-slate-400 dark:text-zinc-500 absolute left-2.5 top-1/2 -translate-y-1/2" />
                    <input
                      type="text"
                      value={p}
                      onChange={(e) => handlePathChange(idx, e.target.value)}
                      placeholder={idx === 0 ? './docs' : idx === 1 ? './architecture/adrs' : '~/Obsidian/Vault'}
                      required
                      className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded pl-8 pr-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-emerald-500 font-mono"
                    />
                  </div>

                  {/* Native Browse Folder Button */}
                  <button
                    type="button"
                    onClick={() => handleBrowseFolder(idx)}
                    className="p-1.5 text-slate-500 hover:text-emerald-600 dark:text-zinc-400 dark:hover:text-emerald-400 rounded bg-slate-100 dark:bg-zinc-800 hover:bg-slate-200 dark:hover:bg-zinc-700 transition flex items-center gap-1 border border-slate-200 dark:border-zinc-700 cursor-pointer select-none"
                    title="Browse Folder..."
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    <span className="text-[10px] font-semibold hidden sm:inline">Browse</span>
                  </button>

                  <input
                    id={`folder-input-${idx}`}
                    type="file"
                    ref={(el) => {
                      if (el) {
                        el.setAttribute('webkitdirectory', '');
                        el.setAttribute('directory', '');
                      }
                      fileInputRefs.current[idx] = el;
                    }}
                    onChange={(e) => handleFolderSelected(idx, e)}
                    className="hidden"
                  />

                  {paths.length > 1 && (
                    <button
                      type="button"
                      onClick={() => handleRemovePath(idx)}
                      className="p-1.5 text-slate-400 hover:text-rose-500 rounded hover:bg-slate-100 dark:hover:bg-zinc-800 transition"
                      title="Remove path"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
              ))}
            </div>
            <p className="text-[10px] text-slate-400 dark:text-zinc-500 mt-1.5">
              Click <strong>Browse</strong> to select a folder from your computer, or type a path manually.
            </p>
          </div>

          <div>
            <label className="block text-slate-600 dark:text-zinc-400 font-medium mb-1">File Glob Patterns</label>
            <input
              type="text"
              value={globPatterns}
              onChange={(e) => setGlobPatterns(e.target.value)}
              placeholder="*.md, *.markdown, *.mdx"
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded px-3 py-1.5 text-slate-900 dark:text-zinc-100 focus:outline-none focus:border-emerald-500 font-mono"
            />
          </div>

          {errorMessage && (
            <div className="p-2.5 rounded text-xs bg-rose-50 dark:bg-rose-950/50 border border-rose-200 dark:border-rose-800/50 text-rose-700 dark:text-rose-400">
              {errorMessage}
            </div>
          )}

          <div className="flex items-center justify-end gap-2 border-t border-slate-200 dark:border-zinc-800 pt-3 mt-4">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 rounded bg-slate-100 dark:bg-zinc-800 hover:bg-slate-200 dark:hover:bg-zinc-700 text-slate-600 dark:text-zinc-400 hover:text-slate-900 dark:hover:text-zinc-200 transition"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="px-4 py-1.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-bold transition disabled:opacity-50 flex items-center gap-1.5"
            >
              {isSubmitting && <Loader2 className="w-3 h-3 animate-spin" />}
              <span>Save Connector</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
