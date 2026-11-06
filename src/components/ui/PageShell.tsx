import type { JSX, ParentProps } from "solid-js";
import styles from "./PageShell.module.css";

interface PageShellProps extends ParentProps {
  title: string;
  notice?: JSX.Element;
  toolbar: JSX.Element;
  panelTitle: string;
  panel: JSX.Element;
  footer: JSX.Element;
}

export default function PageShell(props: PageShellProps) {
  return (
    <div class={styles.shell}>
      <section class={styles.main}>
        <header class={styles.header}>
          <h1 class={styles.title}>{props.title}</h1>
          {props.notice}
          {props.toolbar}
        </header>
        <div class={styles.scroll}>{props.children}</div>
      </section>
      <aside class={styles.panel}>
        <h2 class={styles.panelTitle}>{props.panelTitle}</h2>
        {props.panel}
      </aside>
      <footer class={styles.footer}>{props.footer}</footer>
    </div>
  );
}
