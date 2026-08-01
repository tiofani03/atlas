import React, { useState, useRef } from 'react';
import { X, Loader2, Plus, Trash2, HardDrive, FolderOpen } from 'lucide-react';
import { api } from '../../services/api';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  initialConfig?: {
    id: string;
    path?: string;
    paths?: string[];
  };
}

export const ConfigureLocalGitModal: React.FC<ModalProps> = ({
  isOpen,
  onClose,
  onSuccess,
  initialConfig,
}) => {
  const [id, setId] = useState(initialConfig?.id || 'local-git-main');

  const initialPaths =
    initialConfig?.paths && initialConfig.paths.length > 0
      ? initialConfig.paths
      : initialConfig?.path
      ? initialConfig.path.split(',').map((p) => p.trim()).filter(Boolean)
      : ['./'];

  const [paths, setPaths] = useState<string[]>(initialPaths);
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
      // Fall back to input
    }

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

    const validPaths = paths.map((p) => p.trim()).filter(Boolean);

    if (validPaths.length === 0) {
      setErrorMessage('Please specify at least one repository or workspace path.');
      setIsSubmitting(false);
      return;
    }

    try {
      await api.saveLocalGitConnector({
        id,
        path: validPaths[0],
        paths: validPaths,
      });

      onSuccess();
      onClose();
    } catch (err: unknown) {
      if (err instanceof Error) {
        setErrorMessage(err.message);
      } else {
        setErrorMessage('Failed to save Local Git Connector configuration');
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 dark:bg-black/80 backdrop-blur-xs animate-in fade-in duration-200">
      <div className="bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 rounded-2xl w-full max-w-xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-200 dark:border-zinc-800/80 bg-slate-50/50 dark:bg-zinc-950/40">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-emerald-50 dark:bg-emerald-600/20 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-500/30 flex items-center justify-center font-bold">
              <HardDrive className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-base font-bold text-slate-900 dark:text-zinc-100">
                Configure Local Git Repositories
              </h3>
              <p className="text-xs text-slate-500 dark:text-zinc-400">
                Atlas v0.2 Read-Only Local Git Engine
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-xl text-slate-400 hover:text-slate-600 dark:hover:text-zinc-200 hover:bg-slate-100 dark:hover:bg-zinc-800 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Form Content */}
        <form onSubmit={handleSubmit} className="p-6 space-y-5 overflow-y-auto flex-1">
          {errorMessage && (
            <div className="p-3.5 rounded-xl bg-rose-50 dark:bg-rose-950/50 border border-rose-200 dark:border-rose-800/50 text-rose-700 dark:text-rose-300 text-xs font-medium">
              {errorMessage}
            </div>
          )}

          {/* Connector Identifier */}
          <div className="space-y-1.5">
            <label className="text-xs font-bold text-slate-700 dark:text-zinc-300">
              Connector Identifier
            </label>
            <input
              type="text"
              required
              value={id}
              onChange={(e) => setId(e.target.value)}
              placeholder="e.g. local-git-main"
              className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-xl px-3.5 py-2 text-xs text-slate-900 dark:text-zinc-100 placeholder-slate-400 dark:placeholder-zinc-500 focus:outline-none focus:border-indigo-500 font-mono transition"
            />
            <p className="text-[11px] text-slate-500 dark:text-zinc-500">
              Unique ID for this connector instance in Atlas config.
            </p>
          </div>

          {/* Repository Paths */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-xs font-bold text-slate-700 dark:text-zinc-300">
                Repository & Workspace Paths
              </label>
              <button
                type="button"
                onClick={handleAddPath}
                className="text-[11px] font-semibold text-indigo-600 hover:text-indigo-500 dark:text-indigo-400 dark:hover:text-indigo-300 flex items-center gap-1 bg-indigo-50 dark:bg-indigo-950/60 px-2.5 py-1 rounded-lg border border-indigo-200 dark:border-indigo-800/40 transition"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Add Path</span>
              </button>
            </div>

            <div className="space-y-2 max-h-[220px] overflow-y-auto pr-1">
              {paths.map((pathValue, index) => (
                <div key={index} className="flex items-center gap-2">
                  <div className="relative flex-1">
                    <input
                      type="text"
                      required
                      value={pathValue}
                      onChange={(e) => handlePathChange(index, e.target.value)}
                      placeholder="/Users/username/Projects/my-repo or ~/Projects/"
                      className="w-full bg-slate-50 dark:bg-zinc-950 border border-slate-200 dark:border-zinc-800 rounded-xl pl-3.5 pr-20 py-2 text-xs text-slate-900 dark:text-zinc-100 placeholder-slate-400 dark:placeholder-zinc-500 focus:outline-none focus:border-indigo-500 font-mono transition"
                    />

                    {/* Hidden folder input */}
                    <input
                      type="file"
                      ref={(el) => {
                        fileInputRefs.current[index] = el;
                      }}
                      style={{ display: 'none' }}
                      // @ts-ignore
                      directory=""
                      // @ts-ignore
                      webkitdirectory=""
                      onChange={(e) => handleFolderSelected(index, e)}
                    />

                    <button
                      type="button"
                      onClick={() => handleBrowseFolder(index)}
                      disabled={isPickerOpening}
                      className="absolute right-1.5 top-1/2 -translate-y-1/2 px-2.5 py-1 bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 hover:border-slate-300 dark:hover:border-zinc-700 text-slate-600 dark:text-zinc-300 hover:text-slate-900 dark:hover:text-zinc-100 text-[10px] font-semibold rounded-lg transition flex items-center gap-1 shadow-2xs"
                    >
                      <FolderOpen className="w-3 h-3 text-emerald-500" />
                      <span>Browse</span>
                    </button>
                  </div>

                  {paths.length > 1 && (
                    <button
                      type="button"
                      onClick={() => handleRemovePath(index)}
                      className="p-2 text-slate-400 hover:text-rose-500 dark:hover:text-rose-400 hover:bg-rose-50 dark:hover:bg-rose-950/50 rounded-xl transition border border-transparent hover:border-rose-200 dark:hover:border-rose-800/40"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  )}
                </div>
              ))}
            </div>

            <div className="p-3 bg-slate-50/80 dark:bg-zinc-950/60 border border-slate-200/80 dark:border-zinc-800/80 rounded-xl space-y-1 text-[11px] text-slate-500 dark:text-zinc-400">
              <p className="font-semibold text-slate-700 dark:text-zinc-300">
                Multi-Repo & Workspace Support:
              </p>
              <p>
                • Specify an explicit repository root containing a <code className="font-mono text-emerald-600 dark:text-emerald-400">.git</code> folder.
              </p>
              <p>
                • Or specify a workspace directory (e.g. <code className="font-mono text-emerald-600 dark:text-emerald-400">~/Projects</code>) to auto-discover child Git repositories.
              </p>
            </div>
          </div>

          {/* Action Footer */}
          <div className="flex items-center justify-end gap-3 pt-4 border-t border-slate-200 dark:border-zinc-800/80">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-xl text-xs font-semibold text-slate-600 dark:text-zinc-400 hover:bg-slate-100 dark:hover:bg-zinc-800 transition"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="px-4 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-bold transition flex items-center gap-2 disabled:opacity-50 shadow-md shadow-indigo-500/20 active:scale-95"
            >
              {isSubmitting ? (
                <>
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  <span>Saving Configuration...</span>
                </>
              ) : (
                <span>Save Local Git Connector</span>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
