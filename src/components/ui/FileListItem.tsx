import { Show } from "solid-js";
import styles from "./FileListItem.module.css";

export interface FileInfo {
  path: string;
  name: string;
  size: string;
  dimensions?: string;
  thumbnail?: string;
}

interface FileListItemProps {
  file: FileInfo;
  iconType?: "image" | "video";
  onRemove?: () => void;
}

export default function FileListItem(props: FileListItemProps) {
  const iconPath =
    props.iconType === "video"
      ? "m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.934a.5.5 0 0 0-.777-.416L16 11Z M2 7a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2Z"
      : "M21 3H3a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h18a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Z M8.5 9.5a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3Z M21 15l-5-6L5 21";

  const meta = props.file.dimensions
    ? `${props.file.dimensions} · ${props.file.size}`
    : props.file.size;

  return (
    <div class={styles.item}>
      <div class={styles.thumbnail}>
        <Show
          when={props.file.thumbnail}
          fallback={
            <svg
              class={styles.thumbIcon}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d={iconPath} />
            </svg>
          }
        >
          <img
            src={props.file.thumbnail}
            alt={props.file.name}
            class={styles.thumbImg}
          />
        </Show>
      </div>
      <div class={styles.info}>
        <span class={styles.filename}>{props.file.name}</span>
        <span class={styles.meta}>{meta}</span>
      </div>
      {props.onRemove && (
        <button class={styles.remove} onClick={props.onRemove}>
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}
