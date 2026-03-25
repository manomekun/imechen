import styles from "./ProgressBar.module.css";

interface ProgressBarProps {
  percent: number;
}

export default function ProgressBar(props: ProgressBarProps) {
  return (
    <div class={styles.track}>
      <div class={styles.fill} style={{ width: `${props.percent}%` }} />
    </div>
  );
}
