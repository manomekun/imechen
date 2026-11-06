import { For } from "solid-js";
import { useTheme, type ThemeMode } from "../stores/theme";
import Icon, { type IconName } from "../components/ui/Icon";
import styles from "./Settings.module.css";

const themes: { mode: ThemeMode; label: string; icon: IconName }[] = [
  { mode: "dark", label: "ダーク", icon: "moon" },
  { mode: "light", label: "ライト", icon: "sun" },
  { mode: "system", label: "システム", icon: "monitor" },
];

export default function Settings() {
  const { themeMode, setThemeMode } = useTheme();

  return (
    <div class={styles.page}>
      <h1 class={styles.title}>設定</h1>
      <section class={styles.section}>
        <h2 class={styles.heading}>テーマ</h2>
        <p class={styles.desc}>アプリの外観テーマを選択してください</p>
        <div class={styles.options}>
          <For each={themes}>
            {(t) => (
              <button
                type="button"
                class={styles.option}
                classList={{ [styles.optionActive]: themeMode() === t.mode }}
                onClick={() => setThemeMode(t.mode)}
              >
                <Icon name={t.icon} size={28} class={styles.optionIcon} />
                <span class={styles.optionLabel}>{t.label}</span>
              </button>
            )}
          </For>
        </div>
      </section>
      <span class={styles.version}>imechen v0.1.0</span>
    </div>
  );
}
