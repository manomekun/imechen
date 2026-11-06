import { For } from "solid-js";
import { A, useLocation } from "@solidjs/router";
import appIcon from "../../assets/icon.png";
import Icon, { type IconName } from "../ui/Icon";
import styles from "./Sidebar.module.css";

const navItems: { path: string; icon: IconName; label: string }[] = [
  { path: "/image", icon: "image", label: "画像変換" },
  { path: "/video", icon: "video", label: "動画変換" },
  { path: "/animation", icon: "layers", label: "アニメーション" },
];

export default function Sidebar() {
  const location = useLocation();
  const isActive = (path: string) => location.pathname === path;

  return (
    <aside class={styles.sidebar}>
      <div class={styles.logo}>
        <img src={appIcon} alt="" class={styles.logoImg} />
        <span class={styles.logoText}>imechen</span>
      </div>
      <nav class={styles.nav}>
        <For each={navItems}>
          {(item) => (
            <A
              href={item.path}
              class={styles.navItem}
              classList={{ [styles.navItemActive]: isActive(item.path) }}
            >
              <Icon name={item.icon} size={22} class={styles.navIcon} />
              <span class={styles.navLabel}>{item.label}</span>
            </A>
          )}
        </For>
      </nav>
      <div class={styles.bottom}>
        <A
          href="/settings"
          class={styles.navItem}
          classList={{ [styles.navItemActive]: isActive("/settings") }}
        >
          <Icon name="settings" size={22} class={styles.navIcon} />
          <span class={styles.navLabel}>設定</span>
        </A>
      </div>
    </aside>
  );
}
