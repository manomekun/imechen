import { Show } from "solid-js";
import styles from "./PixelLoader.module.css";

interface PixelLoaderProps {
  visible: boolean;
  label?: string;
  progress?: number;
}

// Colors inspired by the accent palette
const pixelColors = [
  "#F97316", "#FB923C", "#FDBA74", "#FED7AA",
  "#F97316", "#EA580C", "#C2410C", "#FB923C",
  "#FDBA74", "#F97316", "#FB923C", "#FED7AA",
  "#EA580C", "#FDBA74", "#F97316", "#C2410C",
  "#FB923C", "#F97316", "#EA580C", "#FDBA74",
  "#FED7AA", "#C2410C", "#F97316", "#FB923C",
  "#F97316", "#FDBA74", "#EA580C", "#FB923C",
  "#FED7AA", "#F97316", "#C2410C", "#FDBA74",
  "#FB923C", "#FED7AA", "#F97316", "#EA580C",
];

// Pre-generate random scatter directions for each pixel
const scatterDirs = Array.from({ length: 36 }, (_, i) => {
  const angle = (i / 36) * Math.PI * 2 + Math.random() * 0.8;
  const dist = 30 + Math.random() * 50;
  return {
    dx: `${Math.cos(angle) * dist}px`,
    dy: `${Math.sin(angle) * dist}px`,
  };
});

export default function PixelLoader(props: PixelLoaderProps) {
  return (
    <Show when={props.visible}>
      <div class={styles.overlay}>
        <div class={styles.pixelGrid}>
          {pixelColors.map((color, i) => (
            <div
              class={styles.pixel}
              style={{
                "background-color": color,
                "--dx": scatterDirs[i].dx,
                "--dy": scatterDirs[i].dy,
                "animation-delay": `${(Math.floor(i / 6) * 0.08) + ((i % 6) * 0.06)}s`,
              }}
            />
          ))}
        </div>
        <span class={styles.label}>{props.label ?? "変換中..."}</span>
        <Show when={props.progress !== undefined}>
          <span class={styles.sublabel}>{props.progress}%</span>
        </Show>
      </div>
    </Show>
  );
}
