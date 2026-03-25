import { createSignal, onMount, onCleanup } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import styles from "./DropZone.module.css";

interface DropZoneProps {
  label?: string;
  hint?: string;
  icon?: "image" | "video" | "images";
  accept?: string[];
  onFilesSelected?: (paths: string[]) => void;
}

const iconPaths: Record<string, string> = {
  image:
    "M21 3H3a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h18a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Z M8.5 9.5a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3Z M21 15l-5-6L5 21",
  video:
    "m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.934a.5.5 0 0 0-.777-.416L16 11Z M2 7a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2Z",
  images:
    "M18 22H4a2 2 0 0 1-2-2V6 M21 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Z M12 11.5a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3Z M23 16l-4-5-3.5 4.5-2.5-3L7 19",
};

export default function DropZone(props: DropZoneProps) {
  const [isDragOver, setIsDragOver] = createSignal(false);

  // Tauri drag-and-drop event listener
  onMount(async () => {
    const appWindow = getCurrentWebviewWindow();
    const unlisten = await appWindow.onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setIsDragOver(true);
      } else if (event.payload.type === "drop") {
        setIsDragOver(false);
        const paths = event.payload.paths;
        if (paths.length > 0) {
          props.onFilesSelected?.(paths);
        }
      } else {
        // "leave"
        setIsDragOver(false);
      }
    });
    onCleanup(() => unlisten());
  });

  const handleClick = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: props.accept
          ? [{ name: "Files", extensions: props.accept }]
          : [],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        props.onFilesSelected?.(paths);
      }
    } catch (e) {
      console.error("File dialog error:", e);
    }
  };

  return (
    <div
      class={`${styles.dropZone} ${isDragOver() ? styles.dropZoneActive : ""}`}
      onClick={handleClick}
    >
      <svg
        class={styles.icon}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d={iconPaths[props.icon ?? "image"]} />
      </svg>
      <span class={styles.label}>
        {props.label ?? "ここにファイルをドロップまたはクリックして選択"}
      </span>
      <span class={styles.hint}>
        {props.hint ?? "JPEG, PNG, GIF, WebP, BMP, HEIF"}
      </span>
    </div>
  );
}
