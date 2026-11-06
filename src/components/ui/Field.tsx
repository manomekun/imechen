import type { JSX, ParentProps } from "solid-js";
import styles from "./Field.module.css";

interface FieldProps extends ParentProps {
  label: string;
  aside?: JSX.Element;
  dimmed?: boolean;
}

export default function Field(props: FieldProps) {
  return (
    <section class={styles.field} classList={{ [styles.dimmed]: props.dimmed }}>
      <div class={styles.head}>
        <span class={styles.label}>{props.label}</span>
        {props.aside}
      </div>
      {props.children}
    </section>
  );
}

export function ValueBadge(props: ParentProps) {
  return <span class={styles.badge}>{props.children}</span>;
}

export function NoteText(props: ParentProps) {
  return <span class={styles.note}>{props.children}</span>;
}
