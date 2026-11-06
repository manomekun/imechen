import { createSignal, onMount, onCleanup, type Accessor } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export function useFileDrop(onFiles: (paths: string[]) => void): Accessor<boolean> {
  const [isDragOver, setIsDragOver] = createSignal(false);

  let unlisten: (() => void) | undefined;
  onMount(async () => {
    unlisten = await getCurrentWebviewWindow().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setIsDragOver(true);
      } else if (event.payload.type === "drop") {
        setIsDragOver(false);
        const paths = event.payload.paths;
        if (paths.length > 0) onFiles(paths);
      } else {
        setIsDragOver(false);
      }
    });
  });
  onCleanup(() => unlisten?.());

  return isDragOver;
}

export async function pickFiles(
  onFiles: (paths: string[]) => void,
  accept?: string[],
): Promise<void> {
  try {
    const selected = await open({
      multiple: true,
      filters: accept ? [{ name: "Files", extensions: accept }] : [],
    });
    if (selected) {
      onFiles(Array.isArray(selected) ? selected : [selected]);
    }
  } catch (e) {
    console.error("File dialog error:", e);
  }
}
