import { useCallback, useEffect, useState } from "react";
import { FilePlus, Play, Trash2, AlertTriangle } from "lucide-react";
import { pickGgufFile } from "../lib/dialog";
import {
  ipc,
  toAppError,
  type AppError,
  type ModelEntry,
  type RuntimeStatus,
} from "../lib/ipc";
import { ErrorBanner } from "../components/ErrorBanner";
import { formatBytes, formatWhen } from "../lib/format";

export function Models({
  runtime,
  refreshRuntime,
}: {
  runtime: RuntimeStatus;
  refreshRuntime: () => Promise<void>;
}) {
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async () => {
    setModels(await ipc.listModels());
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const addModel = async () => {
    setError(null);
    const selected = await pickGgufFile();
    if (!selected) return;
    try {
      await ipc.addModel(selected);
      await reload();
    } catch (e) {
      setError(toAppError(e));
    }
  };

  const useModel = async (id: string) => {
    setError(null);
    setBusy(true);
    try {
      await ipc.startRuntime(id);
      await reload();
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setBusy(false);
      await refreshRuntime();
    }
  };

  const removeModel = async (id: string) => {
    setError(null);
    await ipc.removeModel(id);
    await reload();
  };

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col gap-4 overflow-y-auto px-8 py-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Models</h1>
          <p className="mt-1 text-sm text-brand-textMuted">
            Omnira uses model files you already have, right where they are. Your
            files are never copied or moved.
          </p>
        </div>
        <button
          onClick={() => void addModel()}
          className="flex items-center gap-2 rounded-lg bg-accent-primary px-4 py-2 text-sm font-medium text-white hover:bg-accent-primary/90"
        >
          <FilePlus size={16} />
          Add model
        </button>
      </header>

      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}

      {models.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-brand-border py-16 text-center">
          <h2 className="text-lg font-medium">No models added yet</h2>
          <p className="max-w-md text-sm text-brand-textMuted">
            Add a .gguf model file from your computer to start chatting. If you
            do not have one yet, you can download GGUF models from the web with
            your browser and add them here.
          </p>
          <button
            onClick={() => void addModel()}
            className="mt-2 flex items-center gap-2 rounded-lg bg-accent-primary px-4 py-2 text-sm font-medium text-white hover:bg-accent-primary/90"
          >
            <FilePlus size={16} />
            Choose a .gguf file
          </button>
        </div>
      ) : (
        <ul className="flex flex-col gap-3">
          {models.map((m) => {
            const active = runtime.model_id === m.id && runtime.state === "ready";
            return (
              <li
                key={m.id}
                className="flex items-center gap-4 rounded-xl border border-brand-border bg-brand-card px-5 py-4"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="truncate font-medium">{m.name}</h3>
                    {active && (
                      <span className="rounded-full bg-accent-success/15 px-2 py-0.5 text-[11px] text-accent-success">
                        In use
                      </span>
                    )}
                    {m.status === "missing" && (
                      <span className="flex items-center gap-1 rounded-full bg-accent-warning/15 px-2 py-0.5 text-[11px] text-accent-warning">
                        <AlertTriangle size={11} />
                        File missing
                      </span>
                    )}
                  </div>
                  <p className="mt-0.5 truncate text-xs text-zinc-600" title={m.path}>
                    {m.path}
                  </p>
                  <p className="mt-1 text-xs text-brand-textMuted">
                    {formatBytes(m.file_size_bytes)} - Last used {formatWhen(m.last_used_at)}
                  </p>
                  {m.status === "missing" && (
                    <p className="mt-1 text-xs text-accent-warning">
                      This file has moved or been deleted. Put it back, or remove
                      this entry.
                    </p>
                  )}
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <button
                    onClick={() => void useModel(m.id)}
                    disabled={m.status !== "ok" || busy || runtime.state === "starting"}
                    className="flex items-center gap-1.5 rounded-lg border border-brand-border px-3 py-1.5 text-xs hover:bg-brand-hover disabled:opacity-40"
                  >
                    <Play size={13} />
                    {active ? "Restart" : "Use"}
                  </button>
                  <button
                    onClick={() => void removeModel(m.id)}
                    className="rounded-lg border border-brand-border p-2 text-brand-textMuted hover:bg-brand-hover hover:text-accent-danger"
                    title="Remove from Omnira (your file is not deleted)"
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <p className="pb-2 text-center text-xs text-zinc-600">
        Removing a model from Omnira only removes it from this list. Your model
        file stays exactly where it is.
      </p>
    </div>
  );
}
