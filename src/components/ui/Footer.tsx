import { Show } from "solid-js";
import GradientButton from "./GradientButton";
import ProgressBar from "./ProgressBar";
import styles from "./Footer.module.css";

interface FooterProps {
  status: string;
  statusError?: boolean;
  progress?: number;
  showProgress?: boolean;
  ctaLabel: string;
  ctaDisabled: boolean;
  onCta: () => void;
}

export default function Footer(props: FooterProps) {
  return (
    <>
      <div class={styles.status}>
        <span class={styles.text} classList={{ [styles.error]: props.statusError }}>
          {props.status}
        </span>
        <Show when={props.showProgress}>
          <div class={styles.bar}>
            <ProgressBar percent={props.progress ?? 0} />
          </div>
        </Show>
      </div>
      <GradientButton onClick={props.onCta} disabled={props.ctaDisabled}>
        {props.ctaLabel}
      </GradientButton>
    </>
  );
}

interface BatchState {
  fileCount: number;
  isConverting: boolean;
  progress: number;
  resultCount: number;
  errorCount: number;
}

export function batchStatus(s: BatchState) {
  const showProgress = s.isConverting || s.progress > 0;
  if (s.isConverting) {
    return { text: `変換中... ${s.progress}%`, error: false, showProgress };
  }
  if (s.progress > 0) {
    if (s.errorCount === 0) return { text: "完了", error: false, showProgress };
    return {
      text:
        s.resultCount === 0
          ? `失敗 (${s.errorCount}件)`
          : `完了 (${s.errorCount}件失敗)`,
      error: true,
      showProgress,
    };
  }
  return { text: `${s.fileCount} ファイル`, error: false, showProgress };
}
