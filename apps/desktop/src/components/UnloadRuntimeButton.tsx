import { useState } from "react";
import { Power } from "lucide-react";
import { ipc, toAppError, type AppError, type RuntimeStatus } from "../lib/ipc";

/**
 * The one visible control for stopping the chat engine. Cancels a model that
 * is still loading, or unloads a running one, and always frees the process.
 * Hidden when nothing is loaded so the empty state stays calm.
 */
export function UnloadRuntimeButton({
  runtime,
  refreshRuntime,
  onBeforeStop,
  onError,
  compact = false,
}: {
  runtime: RuntimeStatus;
  refreshRuntime: () => Promise<void>;
  /** Called first, e.g. to cancel an in-flight generation. */
  onBeforeStop?: () => void;
  onError?: (error: AppError) => void;
  compact?: boolean;
}) {
  const [busy, setBusy] = useState(false);

  const showFor: RuntimeStatus["state"][] = ["ready", "starting", "error"];
  if (!showFor.includes(runtime.state)) return null;

  const label =
    runtime.state === "starting"
      ? "Cancel loading"
      : runtime.state === "ready"
        ? "Unload model"
        : "Stop engine";
  const title =
    runtime.state === "starting"
      ? "Stop loading this model and free the chat engine"
      : "Stop the chat engine and free its memory";

  const stop = async () => {
    setBusy(true);
    onBeforeStop?.();
    try {
      await ipc.stopRuntime();
    } catch (e) {
      onError?.(toAppError(e));
    } finally {
      await refreshRuntime();
      setBusy(false);
    }
  };

  return (
    <button
      type="button"
      onClick={() => void stop()}
      disabled={busy}
      title={title}
      aria-label={label}
      className={`inline-flex items-center gap-1.5 rounded-lg border border-brand-border text-xs text-brand-textMuted hover:bg-brand-hover hover:text-zinc-100 disabled:opacity-50 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent-primary/50 ${
        compact ? "px-2 py-1" : "px-3 py-1.5"
      }`}
    >
      <Power size={13} />
      {busy ? "Stopping..." : label}
    </button>
  );
}
