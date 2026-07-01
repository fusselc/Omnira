import type { RuntimeStatus } from "../lib/ipc";

/**
 * The privacy/status indicator. Copy rule (docs/design-principles.md):
 * the main UI says "Running locally" and never names the accelerator --
 * accelerator details belong to Advanced Diagnostics only.
 */
export function StatusPill({ status }: { status: RuntimeStatus }) {
  const map = {
    ready: { dot: "bg-accent-success", label: "Running locally" },
    starting: { dot: "bg-accent-warning animate-pulse", label: "Starting model..." },
    error: { dot: "bg-accent-danger", label: "Engine problem" },
    stopped: { dot: "bg-zinc-600", label: "No model running" },
  } as const;
  const { dot, label } = map[status.state];

  return (
    <span className="inline-flex items-center gap-2 rounded-full border border-brand-border bg-brand-card px-3 py-1 text-xs text-brand-textMuted">
      <span className={`h-2 w-2 rounded-full ${dot}`} />
      {label}
    </span>
  );
}
