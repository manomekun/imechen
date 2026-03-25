import type { JSX } from "solid-js";
import styles from "./NeuSelect.module.css";

interface NeuSelectProps {
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  style?: JSX.CSSProperties;
}

export default function NeuSelect(props: NeuSelectProps) {
  return (
    <select
      class={styles.select}
      value={props.value}
      onChange={(e) => props.onChange(e.currentTarget.value)}
      style={props.style}
    >
      {props.options.map((opt) => (
        <option value={opt.value}>{opt.label}</option>
      ))}
    </select>
  );
}
