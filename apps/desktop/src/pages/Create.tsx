import { useCallback, useEffect, useState } from "react";
import { ImagePlus, Trash2 } from "lucide-react";
import {
  ipc,
  toAppError,
  type AppError,
  type Generation,
  type ImageRuntimeStatus,
} from "../lib/ipc";
import { ErrorBanner } from "../components/ErrorBanner";
import { formatWhen } from "../lib/format";

const SIZES = [512, 768, 1024] as const;

export function Create() {
  const [prompt, setPrompt] = useState("");
  const [width, setWidth] = useState<number>(768);
  const [height, setHeight] = useState<number>(768);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [status, setStatus] = useState<ImageRuntimeStatus | null>(null);
  const [items, setItems] = useState<Generation[]>([]);

  const refresh = useCallback(async () => {
    setStatus(await ipc.imageRuntimeStatus());
    setItems(await ipc.listGenerations());
  }, []);

  useEffect(() => {
    void refresh().catch((e) => setError(toAppError(e)));
  }, [refresh]);

  const generate = async () => {
    const text = prompt.trim();
    if (!text || busy) return;
    setBusy(true);
    setError(null);
    try {
      await ipc.generateImage(text, width, height);
      setPrompt("");
      await refresh();
    } catch (e) {
      setError(toAppError(e));
      await refresh().catch(() => undefined);
    } finally {
      setBusy(false);
    }
  };

  const remove = async (id: string) => {
    try {
      await ipc.deleteGeneration(id);
      await refresh();
    } catch (e) {
      setError(toAppError(e));
    }
  };

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col gap-6 overflow-y-auto px-8 py-6">
      <header>
        <h1 className="text-xl font-semibold">Create</h1>
        <p className="mt-1 text-sm text-brand-textMuted">
          Generate images locally on this computer. Nothing is uploaded.
        </p>
      </header>

      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}

      <section className="rounded-xl border border-brand-border bg-brand-card p-5">
        <label className="block text-sm">
          <span className="text-brand-textMuted">Prompt</span>
          <textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            rows={4}
            placeholder="Describe the image you want to create…"
            className="mt-2 w-full resize-none rounded-lg border border-brand-border bg-brand-deep px-3 py-2 text-sm outline-none placeholder:text-zinc-700 focus:border-accent-primary/50"
          />
        </label>

        <div className="mt-4 flex flex-wrap items-end gap-4">
          <label className="text-sm">
            <span className="text-brand-textMuted">Width</span>
            <select
              value={width}
              onChange={(e) => setWidth(Number(e.target.value))}
              className="mt-1 block rounded-lg border border-brand-border bg-brand-deep px-3 py-1.5 text-sm outline-none"
            >
              {SIZES.map((s) => (
                <option key={`w-${s}`} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </label>
          <label className="text-sm">
            <span className="text-brand-textMuted">Height</span>
            <select
              value={height}
              onChange={(e) => setHeight(Number(e.target.value))}
              className="mt-1 block rounded-lg border border-brand-border bg-brand-deep px-3 py-1.5 text-sm outline-none"
            >
              {SIZES.map((s) => (
                <option key={`h-${s}`} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </label>
          <button
            onClick={() => void generate()}
            disabled={busy || !prompt.trim()}
            className="ml-auto inline-flex items-center gap-2 rounded-lg bg-accent-primary px-4 py-2 text-sm font-medium text-white hover:bg-accent-primary/90 disabled:opacity-40"
          >
            <ImagePlus size={16} />
            {busy ? "Generating…" : "Generate"}
          </button>
        </div>

        {status && (
          <p className="mt-4 text-xs text-zinc-600">
            {status.available
              ? "Local diffusion worker ready."
              : "Local diffusion worker not installed yet. Generate will explain what is missing. Chat and CUDA LLM runtimes are unaffected."}
          </p>
        )}
      </section>

      <section>
        <h2 className="mb-3 text-sm font-semibold">Recent generations</h2>
        {items.length === 0 ? (
          <div className="rounded-xl border border-dashed border-brand-border px-6 py-10 text-center">
            <p className="text-sm text-brand-textMuted">
              No local images yet. Generations stay under your Omnira data folder.
            </p>
          </div>
        ) : (
          <ul className="grid gap-3 sm:grid-cols-2">
            {items.map((g) => (
              <li
                key={g.id}
                className="rounded-xl border border-brand-border bg-brand-card p-4"
              >
                <p className="line-clamp-3 text-sm text-zinc-100">{g.prompt}</p>
                <p className="mt-2 text-[11px] text-zinc-600">
                  {g.width}×{g.height} · {g.status} · {formatWhen(g.created_at)}
                </p>
                <p className="mt-1 truncate font-mono text-[10px] text-zinc-700" title={g.path}>
                  {g.path}
                </p>
                <button
                  onClick={() => void remove(g.id)}
                  className="mt-3 inline-flex items-center gap-1 rounded px-2 py-1 text-xs text-zinc-500 hover:bg-brand-hover hover:text-accent-danger"
                >
                  <Trash2 size={12} />
                  Delete
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
