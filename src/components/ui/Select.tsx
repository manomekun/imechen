import Icon from "./Icon";
import styles from "./Select.module.css";

interface SelectProps {
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
}

export default function Select(props: SelectProps) {
  return (
    <div class={styles.wrap}>
      <select
        class={styles.select}
        value={props.value}
        onChange={(e) => props.onChange(e.currentTarget.value)}
      >
        {props.options.map((opt) => (
          <option value={opt.value}>{opt.label}</option>
        ))}
      </select>
      <Icon name="chevronDown" size={16} class={styles.chevron} />
    </div>
  );
}
