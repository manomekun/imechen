import { createEffect, type JSX } from "solid-js";
import styles from "./Checkbox.module.css";

interface CheckboxProps {
  checked: boolean;
  indeterminate?: boolean;
  onChange: () => void;
  label?: JSX.Element;
  ariaLabel?: string;
}

export default function Checkbox(props: CheckboxProps) {
  let input!: HTMLInputElement;

  // `indeterminate` is a DOM property only; there is no attribute to bind
  createEffect(() => {
    input.indeterminate = !!props.indeterminate;
  });

  return (
    <label class={styles.wrap} onClick={(e) => e.stopPropagation()}>
      <input
        ref={input}
        type="checkbox"
        class={styles.box}
        checked={props.checked}
        onChange={() => props.onChange()}
        aria-label={props.ariaLabel}
      />
      {props.label && <span class={styles.label}>{props.label}</span>}
    </label>
  );
}
