import type { ParentProps } from "solid-js";
import styles from "./GradientButton.module.css";

interface GradientButtonProps extends ParentProps {
  onClick?: () => void;
  disabled?: boolean;
}

export default function GradientButton(props: GradientButtonProps) {
  return (
    <button
      class={styles.btn}
      onClick={props.onClick}
      disabled={props.disabled}
    >
      {props.children}
    </button>
  );
}
