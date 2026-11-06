import { For, Show, type JSX } from "solid-js";
import Checkbox from "./Checkbox";
import Icon, { type IconName } from "./Icon";
import type { Selection } from "./useSelection";
import styles from "./FileTable.module.css";

export interface FileInfo {
  path: string;
  name: string;
  size: string;
  dimensions?: string;
  thumbnail?: string;
}

export interface Column<T> {
  header: string;
  width: string;
  nowrap?: boolean;
  render: (file: T, index: number) => JSX.Element;
}

export function thumbColumn<T extends FileInfo>(fallbackIcon: IconName): Column<T> {
  return {
    header: "",
    // The thumbnail gives way before the filename does on narrow windows.
    width: "minmax(0, 110px)",
    render: (file) => (
      <div class={styles.thumb}>
        <Show when={file.thumbnail} fallback={<Icon name={fallbackIcon} size={22} />}>
          <img src={file.thumbnail} alt={file.name} class={styles.thumbImg} />
        </Show>
      </div>
    ),
  };
}

export function nameColumn<T extends FileInfo>(): Column<T> {
  return {
    header: "ファイル名",
    width: "minmax(96px, 1fr)",
    render: (file) => (
      <span class={styles.name} title={file.path}>
        {file.name}
      </span>
    ),
  };
}

interface EmptyState {
  icon: IconName;
  label: string;
  hint: string;
}

interface FileTableProps<T extends FileInfo> {
  files: T[];
  columns: Column<T>[];
  selection: Selection;
  isDragOver: boolean;
  empty: EmptyState;
  onPick: () => void;
}

export default function FileTable<T extends FileInfo>(props: FileTableProps<T>) {
  const cols = () => `40px ${props.columns.map((c) => c.width).join(" ")}`;

  return (
    <div class={styles.table} classList={{ [styles.dragOver]: props.isDragOver }}>
      <Show
        when={props.files.length > 0}
        fallback={
          <button type="button" class={styles.empty} onClick={props.onPick}>
            <Icon name={props.empty.icon} size={40} />
            <span class={styles.emptyLabel}>{props.empty.label}</span>
            <span class={styles.emptyHint}>{props.empty.hint}</span>
          </button>
        }
      >
        <div class={`${styles.row} ${styles.head}`} style={{ "--cols": cols() }}>
          <Checkbox
            checked={props.selection.allSelected()}
            indeterminate={props.selection.partial()}
            onChange={props.selection.toggleAll}
            ariaLabel="すべて選択"
          />
          <For each={props.columns}>
            {(col) => (
              <span class={styles.headCell} classList={{ [styles.nowrap]: col.nowrap }}>
                {col.header}
              </span>
            )}
          </For>
        </div>
        <For each={props.files}>
          {(file, i) => (
            <div class={styles.row} style={{ "--cols": cols() }}>
              <Checkbox
                checked={props.selection.isSelected(file.path)}
                onChange={() => props.selection.toggle(file.path)}
                ariaLabel={`${file.name} を選択`}
              />
              <For each={props.columns}>
                {(col) => (
                  <div class={styles.cell} classList={{ [styles.nowrap]: col.nowrap }}>
                    {col.render(file, i())}
                  </div>
                )}
              </For>
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
