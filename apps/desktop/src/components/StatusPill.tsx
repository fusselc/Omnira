import type { RuntimeStatus } from "../lib/ipc";

/**
 * The privacy/status indicator. Copy rule (docs/design-principles.md):
 * the main UI says "Running locally" and names the engine (llama.cpp) so the
 * user can see what is actually running. Performance claims ("GPU
 * accelerated") never appear here; the accelerator detail lives in the
 * tooltip and in Advanced Diagnostics.
 */
export function StatusPill({ status }: { status: RuntimeStatus }) {
  const map = {
    ready: { dot: "bg-accent-success", label: "Running locally" },
    starting: { dot: "bg-accent-warning animate-pulse motion-reduce:animate-none", label: "Starting model..." },
    error: { dot: "bg-accent-danger", label: "Engine problem" },
    stopped: { dot: "bg-zinc-600", label: "No model running" },
  } as const;
  const { dot, label } = map[status.state];
  const engine = status.engine_label;
  const detail = [
    engine ? `Engine: ${engine}` : null,
    status.accelerator_label ? `Mode: ${status.accelerator_label}` : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <span
      className="inline-flex items-center gap-2 rounded-full border border-brand-border bg-brand-card px-3 py-1 text-xs text-brand-textMuted"
      title={detail || undefined}
    >
      <span className={`h-2 w-2 rounded-full ${dot}`} />
      {label}
      {engine && (
        <span className="text-zinc-500" aria-label={`Engine ${engine}`}>
          · {engine}
        </span>
      )}
    </span>
  );
}
