import type { ParentProps } from "solid-js";
import Sidebar from "./Sidebar";
import styles from "./AppLayout.module.css";

export default function AppLayout(props: ParentProps) {
  return (
    <div class={styles.layout}>
      <Sidebar />
      <main class={styles.main}>{props.children}</main>
    </div>
  );
}
