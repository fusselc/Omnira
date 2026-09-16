/**
 * Ref-based lock for click handlers that must not overlap in the same
 * render frame (useState busy flags flush too late for a double-click).
 */
export function acquire(lock: { current: boolean }): boolean {
  if (lock.current) return false;
  lock.current = true;
  return true;
}

export function release(lock: { current: boolean }): void {
  lock.current = false;
}
