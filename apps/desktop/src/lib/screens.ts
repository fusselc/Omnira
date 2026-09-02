/** The MVP screens (AGENTS.md: Chat, Models, Settings, Advanced Diagnostics). */
export type Screen = "chat" | "models" | "settings" | "diagnostics";

/**
 * Screens that stay mounted (hidden, not unmounted) while another screen is
 * shown. Chat owns the in-flight generation stream; unmounting it would abort
 * the response the moment the user peeks at Models or Settings.
 */
export const PERSISTENT_SCREENS: readonly Screen[] = ["chat"];

export function isScreenMounted(screen: Screen, active: Screen): boolean {
  return screen === active || PERSISTENT_SCREENS.includes(screen);
}
