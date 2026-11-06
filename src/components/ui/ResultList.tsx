import { For, Show } from "solid-js";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import Icon from "./Icon";
import styles from "./ResultList.module.css";

export interface OutputItem {
  name: string;
  detail: string;
  path: string;
  fullPath?: string;
}

export interface ConvertError {
  name: string;
  path: string;
  message: string;
}

interface ResultListProps {
  outputs: OutputItem[];
  errors?: ConvertError[];
}

export default function ResultList(props: ResultListProps) {
  return (
    <>
      <Show when={props.outputs.length > 0}>
        <section class={styles.section}>
          <h2 class={styles.title}>出力 ({props.outputs.length})</h2>
          <For each={props.outputs}>
            {(item) => (
              <div class={styles.item}>
                <Icon name="checkCircle" size={18} class={styles.ok} />
                <div class={styles.body}>
                  <span class={styles.name}>{item.name}</span>
                  <span class={styles.detail}>{item.detail}</span>
                  <Show when={item.fullPath}>
                    <span class={styles.fullPath} title={item.fullPath}>
                      {item.fullPath}
                    </span>
                  </Show>
                </div>
                <button
                  type="button"
                  class={styles.reveal}
                  onClick={() => revealItemInDir(item.path)}
                  title="出力先を開く"
                >
                  <Icon name="external" size={14} />
                  出力先を開く
                </button>
              </div>
            )}
          </For>
        </section>
      </Show>
      <Show when={(props.errors?.length ?? 0) > 0}>
        <section class={styles.section}>
          <h2 class={styles.title}>エラー ({props.errors!.length})</h2>
          <For each={props.errors}>
            {(err) => (
              <div class={`${styles.item} ${styles.itemError}`}>
                <Icon name="alertCircle" size={18} class={styles.bad} />
                <div class={styles.body}>
                  <span class={styles.name} title={err.path}>
                    {err.name}
                  </span>
                  <span class={styles.message}>{err.message}</span>
                </div>
              </div>
            )}
          </For>
        </section>
      </Show>
    </>
  );
}
