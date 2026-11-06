import styles from "./Inputs.module.css";

interface TextInputProps {
  value: string;
  onInput: (value: string) => void;
}

export function TextInput(props: TextInputProps) {
  return (
    <input
      type="text"
      class={styles.input}
      value={props.value}
      onInput={(e) => props.onInput(e.currentTarget.value)}
    />
  );
}

interface NumberInputProps {
  value: string | number;
  onInput: (value: string) => void;
  placeholder?: string;
  min?: number;
  max?: number;
  disabled?: boolean;
  ariaLabel?: string;
}

export function NumberInput(props: NumberInputProps) {
  return (
    <input
      type="number"
      class={styles.input}
      value={props.value}
      placeholder={props.placeholder}
      min={props.min}
      max={props.max}
      disabled={props.disabled}
      aria-label={props.ariaLabel}
      onInput={(e) => props.onInput(e.currentTarget.value)}
    />
  );
}

export function DimensionRow(props: {
  width: string;
  height: string;
  onWidth: (v: string) => void;
  onHeight: (v: string) => void;
  disabled?: boolean;
}) {
  return (
    <div class={styles.dims}>
      <span class={styles.unit}>幅</span>
      <NumberInput value={props.width} onInput={props.onWidth} disabled={props.disabled} ariaLabel="幅" />
      <span class={styles.unit}>×</span>
      <span class={styles.unit}>高さ</span>
      <NumberInput value={props.height} onInput={props.onHeight} disabled={props.disabled} ariaLabel="高さ" />
      <span class={styles.unit}>px</span>
    </div>
  );
}
