import Icon from "./Icon";
import styles from "./PathPicker.module.css";

interface PathPickerProps {
  path: string;
  onSelect: () => void;
}

export default function PathPicker(props: PathPickerProps) {
  return (
    <div class={styles.row}>
      <div class={styles.path} title={props.path} onClick={props.onSelect}>
        {props.path}
      </div>
      <button
        type="button"
        class={styles.btn}
        onClick={props.onSelect}
        aria-label="保存先を選択"
      >
        <Icon name="folder" size={20} />
      </button>
    </div>
  );
}
