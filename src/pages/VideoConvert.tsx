import { createSignal, For, Show, onMount, onCleanup } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";
import DropZone from "../components/ui/DropZone";
import GlassCard from "../components/ui/GlassCard";
import NeuSelect from "../components/ui/NeuSelect";
import GradientButton from "../components/ui/GradientButton";
import NeuButton from "../components/ui/NeuButton";
import ProgressBar from "../components/ui/ProgressBar";
import PixelLoader from "../components/ui/PixelLoader";
import FileListItem, { type FileInfo } from "../components/ui/FileListItem";
import {
  getVideoInfo,
  convertVideo,
  ensureFfmpeg,
  formatFileSize,
  formatDuration,
  type ConvertVideoResult,
  type VideoProgress,
} from "../lib/tauri";

const videoFormats = [
  { value: "mp4", label: "MP4" },
  { value: "mov", label: "MOV" },
  { value: "avi", label: "AVI" },
  { value: "webm", label: "WebM" },
  { value: "flv", label: "FLV" },
];

const resolutionPresets = [
  { value: "original", label: "オリジナル" },
  { value: "1080", label: "1080p" },
  { value: "720", label: "720p" },
  { value: "480", label: "480p" },
  { value: "custom", label: "カスタム" },
];

// Module-level: ffmpeg state persists across tab switches
const [ffmpegReady, setFfmpegReady] = createSignal(false);
const [ffmpegLoading, setFfmpegLoading] = createSignal(false);
let ffmpegChecked = false;

async function checkFfmpeg() {
  if (ffmpegChecked) return;
  ffmpegChecked = true;
  try {
    setFfmpegLoading(true);
    await ensureFfmpeg();
    setFfmpegReady(true);
  } catch (e) {
    console.error("ffmpeg setup failed:", e);
    ffmpegChecked = false; // retry next time
  } finally {
    setFfmpegLoading(false);
  }
}

export default function VideoConvert() {
  const [files, setFiles] = createSignal<(FileInfo & { width: number; height: number; duration: number })[]>([]);
  const [outputFormat, setOutputFormat] = createSignal("mp4");
  const [outputDir, setOutputDir] = createSignal("~/Downloads");
  const [quality, setQuality] = createSignal(75);
  const [resolutionPreset, setResolutionPreset] = createSignal("original");
  const [resizeWidth, setResizeWidth] = createSignal("");
  const [resizeHeight, setResizeHeight] = createSignal("");
  const [isConverting, setIsConverting] = createSignal(false);
  const [progress, setProgress] = createSignal(0);
  const [results, setResults] = createSignal<ConvertVideoResult[]>([]);

  onMount(() => checkFfmpeg());

  // Listen for video progress events
  let unlisten: (() => void) | undefined;
  onMount(async () => {
    unlisten = await listen<VideoProgress>("video-progress", (event) => {
      setProgress(Math.round(event.payload.percent));
    });
  });
  onCleanup(() => unlisten?.());

  const handleFilesSelected = async (paths: string[]) => {
    for (const path of paths) {
      try {
        const info = await getVideoInfo(path);
        setFiles((prev) => [
          ...prev,
          {
            path: info.path,
            name: info.name,
            size: formatFileSize(info.size),
            dimensions: `${info.width}×${info.height}`,
            width: info.width,
            height: info.height,
            duration: info.duration_secs,
          },
        ]);
        // Auto-fill resolution from first file
        if (files().length === 0) {
          setResizeWidth(String(info.width));
          setResizeHeight(String(info.height));
        }
      } catch (e) {
        console.error("Failed to get video info:", e);
      }
    }
  };

  const removeFile = (index: number) => {
    setFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const getResizeDimensions = () => {
    const preset = resolutionPreset();
    if (preset === "original") return { w: undefined, h: undefined };
    if (preset === "custom") {
      const w = parseInt(resizeWidth());
      const h = parseInt(resizeHeight());
      return { w: w || undefined, h: h || undefined };
    }
    // Preset: scale height, calculate width from first file's aspect ratio
    const targetH = parseInt(preset);
    const firstFile = files()[0];
    if (firstFile && firstFile.height > 0) {
      const aspect = firstFile.width / firstFile.height;
      return { w: Math.round(targetH * aspect), h: targetH };
    }
    return { w: undefined, h: targetH };
  };

  const yieldToUI = () =>
    new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));

  const handleConvert = async () => {
    const fileList = [...files()];
    if (fileList.length === 0) return;

    setIsConverting(true);
    setProgress(0);
    setResults([]);
    await yieldToUI();

    const { w, h } = getResizeDimensions();

    for (const file of fileList) {
      try {
        const res = await convertVideo({
          source_path: file.path,
          output_format: outputFormat(),
          output_dir: outputDir(),
          quality: quality(),
          resize_width: w,
          resize_height: h,
        });
        setResults((prev) => [...prev, res]);
        setFiles((prev) => prev.filter((f) => f.path !== file.path));
      } catch (e) {
        console.error(`Conversion failed for ${file.name}:`, e);
      }
      await yieldToUI();
    }

    setIsConverting(false);
  };

  const selectOutputDir = async () => {
    try {
      const dir = await open({ directory: true });
      if (dir) setOutputDir(dir as string);
    } catch (e) {
      console.error("Directory dialog error:", e);
    }
  };

  return (
    <>
      <header
        style={{
          display: "flex",
          "justify-content": "space-between",
          "align-items": "center",
        }}
      >
        <h1 style={{ "font-size": "24px", "font-weight": 600, "letter-spacing": "-0.5px" }}>
          動画変換
        </h1>
        <span style={{ "font-size": "13px", color: "var(--text-secondary)" }}>
          動画フォーマットを変換
        </span>
      </header>

      <Show when={ffmpegLoading()}>
        <div
          class="glass"
          style={{
            padding: "12px 16px",
            "border-radius": "var(--radius-md)",
            "font-size": "13px",
            color: "var(--accent)",
            display: "flex",
            "align-items": "center",
            gap: "8px",
          }}
        >
          ffmpeg を準備中...
        </div>
      </Show>

      <div style={{ flex: 1, display: "flex", gap: "24px", "min-height": 0 }}>
        {/* Left Column */}
        <div style={{ flex: 1, display: "flex", "flex-direction": "column", gap: "20px" }}>
          <DropZone
            icon="video"
            label="ここに動画ファイルをドロップまたはクリックして選択"
            hint="MP4, MOV, AVI, WebM, FLV, HEVC"
            onFilesSelected={handleFilesSelected}
          />

          <GlassCard title="変換設定">
            {/* Output Format */}
            <div style={{ display: "flex", gap: "12px", "align-items": "center" }}>
              <span style={{ "font-size": "12px", "font-weight": 500, color: "var(--text-secondary)", "letter-spacing": "0.5px" }}>
                出力形式
              </span>
              <NeuSelect value={outputFormat()} onChange={setOutputFormat} options={videoFormats} style={{ width: "160px" }} />
            </div>

            {/* Save path */}
            <div style={{ display: "flex", gap: "12px", "align-items": "center" }}>
              <span style={{ "font-size": "12px", "font-weight": 500, color: "var(--text-secondary)", "letter-spacing": "0.5px" }}>
                保存先
              </span>
              <div
                class="glass"
                style={{ flex: 1, padding: "10px 14px", "border-radius": "var(--radius-md)", "font-size": "13px", color: "var(--text-muted)", cursor: "pointer" }}
                onClick={selectOutputDir}
              >
                {outputDir()}
              </div>
              <button
                class="neu"
                style={{ width: "36px", height: "36px", "border-radius": "var(--radius-sm)", border: "1px solid var(--glass-border)", display: "flex", "align-items": "center", "justify-content": "center", cursor: "pointer" }}
                onClick={selectOutputDir}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-secondary)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M5 19a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h4l2 2h4a2 2 0 0 1 2 2v1M5 19h14a2 2 0 0 0 2-2v-5a2 2 0 0 0-2-2H9a2 2 0 0 0-2 2v5a2 2 0 0 1-2 2Z" />
                </svg>
              </button>
            </div>

            {/* Resolution */}
            <div style={{ display: "flex", "flex-direction": "column", gap: "10px" }}>
              <span style={{ "font-size": "12px", "font-weight": 600, color: "var(--text-secondary)", "letter-spacing": "0.5px" }}>
                解像度
              </span>
              <div style={{ display: "flex", gap: "12px", "align-items": "center", "flex-wrap": "wrap" }}>
                <NeuSelect
                  value={resolutionPreset()}
                  onChange={(v) => {
                    setResolutionPreset(v);
                    if (v === "original" && files().length > 0) {
                      setResizeWidth(String(files()[0].width));
                      setResizeHeight(String(files()[0].height));
                    } else if (v !== "custom" && v !== "original") {
                      const targetH = parseInt(v);
                      const f = files()[0];
                      if (f && f.height > 0) {
                        setResizeWidth(String(Math.round(targetH * f.width / f.height)));
                        setResizeHeight(String(targetH));
                      }
                    }
                  }}
                  options={resolutionPresets}
                  style={{ width: "160px" }}
                />
                <Show when={resolutionPreset() === "custom" || resolutionPreset() !== "original"}>
                  <span style={{ "font-size": "11px", color: "var(--text-muted)" }}>自動入力 →</span>
                  <input
                    class="glass"
                    type="number"
                    placeholder="幅"
                    value={resizeWidth()}
                    onInput={(e) => setResizeWidth(e.currentTarget.value)}
                    disabled={resolutionPreset() !== "custom"}
                    style={{
                      width: "80px", padding: "10px 14px", "border-radius": "var(--radius-md)", "font-size": "13px",
                      color: "var(--text-primary)", background: "var(--bg-glass)", border: "1px solid var(--glass-border)", "font-family": "var(--font-ui)",
                    }}
                  />
                  <span style={{ color: "var(--text-muted)", "font-size": "14px" }}>×</span>
                  <input
                    class="glass"
                    type="number"
                    placeholder="高さ"
                    value={resizeHeight()}
                    onInput={(e) => setResizeHeight(e.currentTarget.value)}
                    disabled={resolutionPreset() !== "custom"}
                    style={{
                      width: "80px", padding: "10px 14px", "border-radius": "var(--radius-md)", "font-size": "13px",
                      color: "var(--text-primary)", background: "var(--bg-glass)", border: "1px solid var(--glass-border)", "font-family": "var(--font-ui)",
                    }}
                  />
                </Show>
              </div>
            </div>

            {/* Quality */}
            <div style={{ display: "flex", "flex-direction": "column", gap: "10px" }}>
              <div style={{ display: "flex", "justify-content": "space-between", "align-items": "center" }}>
                <span style={{ "font-size": "12px", "font-weight": 600, color: "var(--text-secondary)", "letter-spacing": "0.5px" }}>
                  画質
                </span>
                <span class="mono" style={{ "font-size": "12px", color: "var(--accent)" }}>{quality()}%</span>
              </div>
              <input
                type="range"
                min="1"
                max="100"
                value={quality()}
                onInput={(e) => setQuality(parseInt(e.currentTarget.value))}
                style={{ width: "100%", "accent-color": "var(--accent)" }}
              />
            </div>
          </GlassCard>

          {/* Action Area */}
          <div style={{ display: "flex", "flex-direction": "column", gap: "16px" }}>
            <div style={{ display: "flex", gap: "12px", "align-items": "center" }}>
              <GradientButton
                onClick={handleConvert}
                disabled={files().length === 0 || isConverting() || !ffmpegReady()}
              >
                変換開始
              </GradientButton>
              <NeuButton disabled={!isConverting()}>一時停止</NeuButton>
              <NeuButton disabled={!isConverting()}>キャンセル</NeuButton>
            </div>
            <Show when={isConverting() || progress() > 0}>
              <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                <div style={{ display: "flex", "justify-content": "space-between" }}>
                  <span style={{ "font-size": "12px", color: "var(--text-secondary)" }}>
                    {isConverting() ? "変換中..." : "完了"}
                  </span>
                  <span class="mono" style={{ "font-size": "12px", color: "var(--accent)" }}>{progress()}%</span>
                </div>
                <ProgressBar percent={progress()} />
              </div>
            </Show>
          </div>
        </div>

        {/* Right Column */}
        <div style={{ width: "320px", display: "flex", "flex-direction": "column", gap: "20px", "min-height": 0 }}>
          <GlassCard
            title={`選択ファイル (${files().length})`}
            style={{ flex: "1", overflow: "auto", "min-height": "0" }}
          >
            <Show
              when={files().length > 0}
              fallback={
                <div style={{ flex: 1, display: "flex", "flex-direction": "column", "align-items": "center", "justify-content": "center", gap: "8px", padding: "32px 0" }}>
                  <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="2">
                    <path d="m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.934a.5.5 0 0 0-.777-.416L16 11Z M2 7a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2Z" />
                  </svg>
                  <span style={{ "font-size": "13px", color: "var(--text-muted)" }}>ファイル未選択</span>
                </div>
              }
            >
              <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                <For each={files()}>
                  {(file, i) => (
                    <FileListItem file={{ ...file, dimensions: `${file.dimensions} · ${formatDuration(file.duration)}` }} iconType="video" onRemove={() => removeFile(i())} />
                  )}
                </For>
              </div>
            </Show>
          </GlassCard>

          <Show when={results().length > 0}>
            <GlassCard title={`出力 (${results().length})`} style={{ "max-height": "280px", overflow: "auto" }}>
              <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                <For each={results()}>
                  {(res) => (
                    <div style={{ display: "flex", gap: "10px", "align-items": "center", padding: "8px 12px", background: "var(--bg-glass)", border: "1px solid var(--glass-border)", "border-radius": "var(--radius-sm)" }}>
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2" style={{ "flex-shrink": 0 }}>
                        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                        <path d="m9 11 3 3L22 4" />
                      </svg>
                      <div style={{ flex: 1, "min-width": 0, display: "flex", "flex-direction": "column", gap: "2px" }}>
                        <span style={{ "font-size": "12px", "font-weight": 500, overflow: "hidden", "text-overflow": "ellipsis", "white-space": "nowrap" }}>
                          {res.output_path.split("/").pop()}
                        </span>
                        <span class="mono" style={{ "font-size": "10px", color: "var(--success)" }}>
                          {formatFileSize(res.original_size)} → {formatFileSize(res.output_size)}
                        </span>
                      </div>
                      <button
                        style={{ background: "none", border: "none", cursor: "pointer", padding: "4px", display: "flex", color: "var(--accent)" }}
                        onClick={() => revealItemInDir(res.output_path)}
                        title="出力先を開く"
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <path d="M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                        </svg>
                      </button>
                    </div>
                  )}
                </For>
              </div>
            </GlassCard>
          </Show>
        </div>
      </div>

      <PixelLoader visible={isConverting()} label="動画変換中..." progress={progress()} />
    </>
  );
}
