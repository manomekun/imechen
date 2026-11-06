import Icon from "./Icon";
import styles from "./Toolbar.module.css";

interface ToolbarProps {
  onAdd: () => void;
  onDelete: () => void;
  deleteDisabled: boolean;
}

export default function Toolbar(props: ToolbarProps) {
  return (
    <div class={styles.toolbar}>
      <button type="button" class={styles.outline} onClick={props.onAdd}>
        <Icon name="plus" size={18} />
        ファイルを追加
      </button>
      <button
        type="button"
        class={styles.iconBtn}
        onClick={props.onDelete}
        disabled={props.deleteDisabled}
        aria-label="選択したファイルを削除"
        title="選択したファイルを削除"
      >
        <Icon name="trash" size={18} />
      </button>
    </div>
  );
}
