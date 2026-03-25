import type { ParentProps } from "solid-js";
import styles from "./GlassCard.module.css";

interface GlassCardProps extends ParentProps {
  title?: string;
  style?: Record<string, string>;
}

export default function GlassCard(props: GlassCardProps) {
  return (
    <div class={styles.card} style={props.style}>
      {props.title && <span class={styles.title}>{props.title}</span>}
      {props.children}
    </div>
  );
}
