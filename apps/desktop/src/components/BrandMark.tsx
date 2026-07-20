export function BrandMark({ className = "", size = "md" }: { className?: string; size?: "sm" | "md" | "lg" }) {
  const sizeClasses = {
    sm: "h-8 w-8 text-sm rounded-lg",
    md: "h-12 w-12 text-lg rounded-xl",
    lg: "h-16 w-16 text-2xl rounded-2xl",
  };

  return (
    <div
      className={`flex select-none items-center justify-center font-bold tracking-wider text-accent-primary border border-accent-primary/20 bg-gradient-to-br from-accent-primary/10 to-accent-secondary/5 shadow-inner ${sizeClasses[size]} ${className}`}
      aria-label="Omnira Brand Mark"
    >
      O
    </div>
  );
}
