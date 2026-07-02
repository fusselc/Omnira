import { AlertTriangle, X } from "lucide-react";
import type { AppError } from "../lib/ipc";

/** Friendly error presentation: message + suggested action, taxonomy-driven. */
export function ErrorBanner({
  error,
  onDismiss,
}: {
  error: AppError;
  onDismiss?: () => void;
}) {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-accent-danger/30 bg-accent-danger/10 px-4 py-3 text-sm">
      <AlertTriangle size={16} className="mt-0.5 shrink-0 text-accent-danger" />
      <div className="flex-1">
        <p className="font-medium text-zinc-100">{error.message}</p>
        <p className="mt-0.5 text-brand-textMuted">{error.suggested_action}</p>
      </div>
      {onDismiss && (
        <button
          onClick={onDismiss}
          className="shrink-0 rounded p-1 text-brand-textMuted hover:bg-brand-hover hover:text-zinc-100"
          aria-label="Dismiss"
        >
          <X size={14} />
        </button>
      )}
    </div>
  );
}
