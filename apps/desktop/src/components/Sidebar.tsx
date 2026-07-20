import { MessageSquare, Boxes, Settings as SettingsIcon, Activity } from "lucide-react";
import { BrandMark } from "./BrandMark";

export type Screen = "chat" | "models" | "settings" | "diagnostics";

const items: { id: Screen; label: string; icon: typeof MessageSquare }[] = [
  { id: "chat", label: "Chat", icon: MessageSquare },
  { id: "models", label: "Models", icon: Boxes },
  { id: "settings", label: "Settings", icon: SettingsIcon },
  { id: "diagnostics", label: "Advanced Diagnostics", icon: Activity },
];

export function Sidebar({
  active,
  onSelect,
}: {
  active: Screen;
  onSelect: (s: Screen) => void;
}) {
  return (
    <nav className="flex w-56 shrink-0 flex-col border-r border-brand-border bg-brand-card">
      <div className="flex items-center gap-2 px-5 py-5">
        <BrandMark size="sm" />
        <span className="text-lg font-semibold tracking-tight">Omnira</span>
      </div>
      <div className="flex flex-1 flex-col gap-1 px-3">
        {items.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => onSelect(id)}
            className={`flex items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors ${
              active === id
                ? "bg-accent-primary/15 text-accent-primary"
                : "text-brand-textMuted hover:bg-brand-hover hover:text-zinc-100"
            }`}
          >
            <Icon size={16} />
            {label}
          </button>
        ))}
      </div>
      <p className="px-5 py-4 text-[11px] leading-snug text-zinc-600">
        Private by default. Nothing leaves your computer.
      </p>
    </nav>
  );
}
