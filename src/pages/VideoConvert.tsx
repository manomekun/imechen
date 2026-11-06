import { createSignal, Show, onMount, onCleanup } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import PageShell from "../components/ui/PageShell";
import Toolbar from "../components/ui/Toolbar";
import FileTable, {
  nameColumn,
  thumbColumn,
  type Column,
  type FileInfo,
} from "../components/ui/FileTable";
import ResultList, { type ConvertError } from "../components/ui/ResultList";
import Footer, { batchStatus } from "../components/ui/Footer";
import Field, { ValueBadge } from "../components/ui/Field";
import Select from "../components/ui/Select";
import { DimensionRow } from "../components/ui/Inputs";
import Slider from "../components/ui/Slider";
import PathPicker from "../components/ui/PathPicker";
import PixelLoader from "../components/ui/PixelLoader";
import { useFileDrop, pickFiles } from "../components/ui/useFileDrop";
import { useSelection } from "../components/ui/useSelection";
import {
  getVideoInfo,
  convertVideo,
  ensureFfmpeg,
  formatFileSize,
  formatDuration,
  type ConvertVideoResult,
  type VideoProgress,
} from "../lib/tauri";
import styles from "./VideoConvert.module.css";

type VideoFile = FileInfo & { width: number; height: number; duration: number };

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

const columns: Column<VideoFile>[] = [
  thumbColumn("video"),
  nameColumn(),
  { header: "解像度", width: "88px", nowrap: true, render: (f) => f.dimensions },
  { header: "長さ", width: "52px", nowrap: true, render: (f) => formatDuration(f.duration) },
  { header: "ファイルサイズ", width: "96px", nowrap: true, render: (f) => f.size },
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
  const [files, setFiles] = createSignal<VideoFile[]>([]);
  const [outputFormat, setOutputFormat] = createSignal("mp4");
  const [outputDir, setOutputDir] = createSignal("~/Downloads");
  const [quality, setQuality] = createSignal(75);
  const [resolutionPreset, setResolutionPreset] = createSignal("original");
  const [resizeWidth, setResizeWidth] = createSignal("");
  const [resizeHeight, setResizeHeight] = createSignal("");
  const [isConverting, setIsConverting] = createSignal(false);
  const [progress, setProgress] = createSignal(0);
  const [results, setResults] = createSignal<ConvertVideoResult[]>([]);
  const [errors, setErrors] = createSignal<ConvertError[]>([]);

  const selection = useSelection(files, setFiles);

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

  const isDragOver = useFileDrop(handleFilesSelected);

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
    setErrors([]);
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
        const message = e instanceof Error ? e.message : String(e);
        console.error(`Conversion failed for ${file.name}:`, e);
        setErrors((prev) => [
          ...prev,
          { name: file.name, path: file.path, message },
        ]);
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

  const handlePresetChange = (v: string) => {
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
  };

  const status = () =>
    batchStatus({
      fileCount: files().length,
      isConverting: isConverting(),
      progress: progress(),
      resultCount: results().length,
      errorCount: errors().length,
    });

  return (
    <>
      <PageShell
        title="動画変換"
        notice={
          <Show when={ffmpegLoading()}>
            <div class={styles.notice}>ffmpeg を準備中...</div>
          </Show>
        }
        toolbar={
          <Toolbar
            onAdd={() => pickFiles(handleFilesSelected)}
            onDelete={selection.removeSelected}
            deleteDisabled={selection.count() === 0}
          />
        }
        panelTitle="書き出し設定"
        panel={
          <>
            <Field label="出力形式">
              <Select value={outputFormat()} onChange={setOutputFormat} options={videoFormats} />
            </Field>
            <Field label="解像度">
              <Select
                value={resolutionPreset()}
                onChange={handlePresetChange}
                options={resolutionPresets}
              />
              <Show when={resolutionPreset() === "custom" || resolutionPreset() !== "original"}>
                <DimensionRow
                  width={resizeWidth()}
                  height={resizeHeight()}
                  onWidth={setResizeWidth}
                  onHeight={setResizeHeight}
                  disabled={resolutionPreset() !== "custom"}
                />
              </Show>
            </Field>
            <Field label="画質" aside={<ValueBadge>{quality()}%</ValueBadge>}>
              <Slider value={quality()} onChange={setQuality} />
            </Field>
            <Field label="保存先">
              <PathPicker path={outputDir()} onSelect={selectOutputDir} />
            </Field>
          </>
        }
        footer={
          <Footer
            status={status().text}
            statusError={status().error}
            showProgress={status().showProgress}
            progress={progress()}
            ctaLabel="変換する"
            ctaDisabled={files().length === 0 || isConverting() || !ffmpegReady()}
            onCta={handleConvert}
          />
        }
      >
        <FileTable
          files={files()}
          columns={columns}
          selection={selection}
          isDragOver={isDragOver()}
          empty={{
            icon: "video",
            label: "ここに動画ファイルをドロップまたはクリックして選択",
            hint: "MP4, MOV, AVI, WebM, FLV, HEVC",
          }}
          onPick={() => pickFiles(handleFilesSelected)}
        />
        <ResultList
          outputs={results().map((res) => ({
            name: res.output_path.split("/").pop() ?? res.output_path,
            detail: `${formatFileSize(res.original_size)} → ${formatFileSize(res.output_size)}`,
            path: res.output_path,
          }))}
          errors={errors()}
        />
      </PageShell>
      <PixelLoader visible={isConverting()} label="動画変換中..." progress={progress()} />
    </>
  );
}
