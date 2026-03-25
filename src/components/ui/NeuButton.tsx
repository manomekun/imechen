import type { ParentProps } from "solid-js";
import styles from "./NeuButton.module.css";

interface NeuButtonProps extends ParentProps {
  onClick?: () => void;
  disabled?: boolean;
}

export default function NeuButton(props: NeuButtonProps) {
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
