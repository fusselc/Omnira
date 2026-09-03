/**
 * The in-app Omnira mark. Draws the same ring-and-dot "O" as the packaged app
 * icon (src-tauri/icons/omnira-icon.svg) so the window, taskbar, and in-app
 * branding match.
 */
export function BrandMark({ className = "", size = "md" }: { className?: string; size?: "sm" | "md" | "lg" }) {
  const sizeClasses = {
    sm: "h-8 w-8 rounded-lg",
    md: "h-12 w-12 rounded-xl",
    lg: "h-16 w-16 rounded-2xl",
  };

  return (
    <div
      className={`flex select-none items-center justify-center border border-accent-primary/20 bg-gradient-to-br from-accent-primary/10 to-accent-secondary/5 shadow-inner ${sizeClasses[size]} ${className}`}
      role="img"
      aria-label="Omnira"
    >
      <svg viewBox="0 0 100 100" className="h-[62%] w-[62%]" aria-hidden="true">
        <defs>
          <linearGradient id="omnira-ring" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#818cf8" />
            <stop offset="0.55" stopColor="#6366f1" />
            <stop offset="1" stopColor="#a855f7" />
          </linearGradient>
        </defs>
        <circle cx="50" cy="50" r="36" fill="none" stroke="url(#omnira-ring)" strokeWidth="14" />
        <circle cx="50" cy="50" r="9" fill="url(#omnira-ring)" />
      </svg>
    </div>
  );
}
