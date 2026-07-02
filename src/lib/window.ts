import { getCurrentWindow } from "@tauri-apps/api/window";

export function startWindowDrag(event: PointerEvent) {
  if (event.button !== 0) return;

  const target = event.target;
  if (!(target instanceof Element)) return;

  const interactive = target.closest("button, input, textarea, select, a");
  const dragClickable = target.closest("[data-drag-click]");
  if (interactive && !dragClickable) return;

  void getCurrentWindow().startDragging();
}
