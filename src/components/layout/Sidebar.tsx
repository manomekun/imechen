import { A, useLocation } from "@solidjs/router";
import appIcon from "../../assets/icon.png";
import styles from "./Sidebar.module.css";

const navItems = [
  { path: "/image", icon: "image", label: "画像変換" },
  { path: "/video", icon: "video", label: "動画変換" },
  { path: "/animation", icon: "film", label: "アニメーション" },
  { path: "/settings", icon: "settings", label: "設定" },
] as const;

// Lucide icon SVG paths
const icons: Record<string, string> = {
  image:
    "M21 3H3a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h18a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2ZM8.5 9.5a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3Zm13.5 9-4.5-6-3.5 4.5-2.5-3L3 18.5",
  video:
    "m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.934a.5.5 0 0 0-.777-.416L16 11ZM2 7a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2Z",
  film: "M7 2v20M17 2v20M2 12h20M2 7h5M2 17h5M17 17h5M17 7h5M2 5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2Z",
  settings:
    "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
};

function Icon(props: { name: string; class?: string }) {
  return (
    <svg
      class={props.class}
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d={icons[props.name]} />
    </svg>
  );
}

export default function Sidebar() {
  const location = useLocation();

  return (
    <aside class={styles.sidebar}>
      <div class={styles.logo}>
        <img src={appIcon} alt="imechen" class={styles.logoImg} />
        <span class={styles.logoText}>imechen</span>
      </div>
      <nav class={styles.nav}>
        {navItems.map((item) => (
          <A
            href={item.path}
            class={`${styles.navItem} ${
              location.pathname === item.path ? styles.navItemActive : ""
            }`}
          >
            <Icon name={item.icon} class={styles.navIcon} />
            <span class={styles.navLabel}>{item.label}</span>
          </A>
        ))}
      </nav>
    </aside>
  );
}
