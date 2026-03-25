import { createSignal, For, Show } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import DropZone from "../components/ui/DropZone";
import GlassCard from "../components/ui/GlassCard";
import NeuSelect from "../components/ui/NeuSelect";
import GradientButton from "../components/ui/GradientButton";
import NeuButton from "../components/ui/NeuButton";
import ProgressBar from "../components/ui/ProgressBar";
import PixelLoader from "../components/ui/PixelLoader";
import FileListItem, { type FileInfo } from "../components/ui/FileListItem";
import {
  getImageInfo,
  convertImage,
  formatFileSize,
  type ConvertImageResult,
} from "../lib/tauri";

const imageFormats = [
  { value: "png", label: "PNG" },
  { value: "jpeg", label: "JPEG" },
  { value: "gif", label: "GIF" },
  { value: "webp", label: "WebP" },
  { value: "bmp", label: "BMP" },
];

const qualitySupported = new Set(["jpeg", "webp", "png"]);

export default function ImageConvert() {
  const [files, setFiles] = createSignal<FileInfo[]>([]);
  const [outputFormat, setOutputFormat] = createSignal("png");
  const [outputDir, setOutputDir] = createSignal("~/Downloads");
  const [quality, setQuality] = createSignal(85);
  const [resizeWidth, setResizeWidth] = createSignal("");
  const [resizeHeight, setResizeHeight] = createSignal("");
  const [keepAspect, setKeepAspect] = createSignal(true);
  const [isConverting, setIsConverting] = createSignal(false);
  const [progress, setProgress] = createSignal(0);
  const [results, setResults] = createSignal<ConvertImageResult[]>([]);

  const handleFilesSelected = async (paths: string[]) => {
    const infos: FileInfo[] = [];
    for (const path of paths) {
      try {
        const info = await getImageInfo(path);
        infos.push({
          path: info.path,
          name: info.name,
          size: formatFileSize(info.size),
          dimensions: `${info.width}×${info.height}`,
          thumbnail: info.thumbnail,
        });
      } catch (e) {
        console.error("Failed to get image info:", e);
      }
    }
    setFiles((prev) => [...prev, ...infos]);
  };

  const removeFile = (index: number) => {
    setFiles((prev) => prev.filter((_, i) => i !== index));
  };

  // Yield to the browser so Solid.js can render pending state changes
  const yieldToUI = () =>
    new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));

  const handleConvert = async () => {
    const fileList = [...files()];
    if (fileList.length === 0) return;

    setIsConverting(true);
    setProgress(0);
    setResults([]);

    // Wait for the loader to render before starting heavy work
    await yieldToUI();

    const total = fileList.length;
    for (let i = 0; i < total; i++) {
      const file = fileList[i];
      try {
        const res = await convertImage({
          source_path: file.path,
          output_format: outputFormat(),
          output_dir: outputDir(),
          quality: quality(),
          resize_width: resizeWidth() ? parseInt(resizeWidth()) : undefined,
          resize_height: resizeHeight() ? parseInt(resizeHeight()) : undefined,
          keep_aspect_ratio: keepAspect(),
        });
        setResults((prev) => [...prev, res]);
        setFiles((prev) => prev.filter((f) => f.path !== file.path));
      } catch (e) {
        console.error(`Conversion failed for ${file.name}:`, e);
      }
      setProgress(Math.round(((i + 1) / total) * 100));
      // Yield between files so progress updates render
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
        <h1
          style={{
            "font-size": "24px",
            "font-weight": 600,
            "letter-spacing": "-0.5px",
          }}
        >
          画像変換
        </h1>
        <span style={{ "font-size": "13px", color: "var(--text-secondary)" }}>
          画像フォーマットを変換
        </span>
      </header>

      <div style={{ flex: 1, display: "flex", gap: "24px", "min-height": 0 }}>
        {/* Left Column */}
        <div
          style={{
            flex: 1,
            display: "flex",
            "flex-direction": "column",
            gap: "20px",
          }}
        >
          <DropZone
            icon="image"
            label="ここにファイルをドロップまたはクリックして選択"
            hint="JPEG, PNG, GIF, WebP, BMP, HEIF"
            onFilesSelected={handleFilesSelected}
          />

          <GlassCard title="変換設定">
            {/* Output Format */}
            <div
              style={{
                display: "flex",
                gap: "12px",
                "align-items": "center",
              }}
            >
              <span
                style={{
                  "font-size": "12px",
                  "font-weight": 500,
                  color: "var(--text-secondary)",
                  "letter-spacing": "0.5px",
                }}
              >
                出力形式
              </span>
              <NeuSelect
                value={outputFormat()}
                onChange={setOutputFormat}
                options={imageFormats}
                style={{ width: "160px" }}
              />
            </div>

            {/* Save path */}
            <div
              style={{
                display: "flex",
                gap: "12px",
                "align-items": "center",
              }}
            >
              <span
                style={{
                  "font-size": "12px",
                  "font-weight": 500,
                  color: "var(--text-secondary)",
                  "letter-spacing": "0.5px",
                }}
              >
                保存先
              </span>
              <div
                class="glass"
                style={{
                  flex: 1,
                  padding: "10px 14px",
                  "border-radius": "var(--radius-md)",
                  "font-size": "13px",
                  color: "var(--text-muted)",
                  cursor: "pointer",
                }}
                onClick={selectOutputDir}
              >
                {outputDir()}
              </div>
              <button
                class="neu"
                style={{
                  width: "36px",
                  height: "36px",
                  "border-radius": "var(--radius-sm)",
                  border: "1px solid var(--glass-border)",
                  display: "flex",
                  "align-items": "center",
                  "justify-content": "center",
                  cursor: "pointer",
                }}
                onClick={selectOutputDir}
              >
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="var(--text-secondary)"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d="M5 19a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h4l2 2h4a2 2 0 0 1 2 2v1M5 19h14a2 2 0 0 0 2-2v-5a2 2 0 0 0-2-2H9a2 2 0 0 0-2 2v5a2 2 0 0 1-2 2Z" />
                </svg>
              </button>
            </div>

            {/* Resize */}
            <div
              style={{
                display: "flex",
                "flex-direction": "column",
                gap: "10px",
              }}
            >
              <span
                style={{
                  "font-size": "12px",
                  "font-weight": 600,
                  color: "var(--text-secondary)",
                  "letter-spacing": "0.5px",
                }}
              >
                リサイズ（任意）
              </span>
              <div
                style={{
                  display: "flex",
                  gap: "12px",
                  "align-items": "center",
                }}
              >
                <input
                  class="glass"
                  type="number"
                  placeholder="幅"
                  value={resizeWidth()}
                  onInput={(e) => setResizeWidth(e.currentTarget.value)}
                  style={{
                    width: "120px",
                    padding: "10px 14px",
                    "border-radius": "var(--radius-md)",
                    "font-size": "13px",
                    color: "var(--text-primary)",
                    background: "var(--bg-glass)",
                    border: "1px solid var(--glass-border)",
                    "font-family": "var(--font-ui)",
                  }}
                />
                <span style={{ color: "var(--text-muted)", "font-size": "14px" }}>×</span>
                <input
                  class="glass"
                  type="number"
                  placeholder="高さ"
                  value={resizeHeight()}
                  onInput={(e) => setResizeHeight(e.currentTarget.value)}
                  style={{
                    width: "120px",
                    padding: "10px 14px",
                    "border-radius": "var(--radius-md)",
                    "font-size": "13px",
                    color: "var(--text-primary)",
                    background: "var(--bg-glass)",
                    border: "1px solid var(--glass-border)",
                    "font-family": "var(--font-ui)",
                  }}
                />
                <button
                  style={{
                    display: "flex",
                    "align-items": "center",
                    gap: "6px",
                    padding: "6px 10px",
                    "border-radius": "var(--radius-full)",
                    background: keepAspect()
                      ? "var(--accent-tint)"
                      : "transparent",
                    border: `1px solid ${keepAspect() ? "var(--accent)" : "var(--glass-border)"}`,
                    color: keepAspect()
                      ? "var(--accent)"
                      : "var(--text-muted)",
                    "font-size": "10px",
                    "font-weight": 500,
                    "font-family": "var(--font-ui)",
                    cursor: "pointer",
                  }}
                  onClick={() => setKeepAspect((v) => !v)}
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <path
                      d={
                        keepAspect()
                          ? "M17 11V7a4 4 0 0 0-8 0v4M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2Z"
                          : "M17 11V7a4 4 0 0 0-4-4 4 4 0 0 0-4 4M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2Z"
                      }
                    />
                  </svg>
                  縦横比維持
                </button>
              </div>
            </div>

            {/* Quality */}
            <div
              style={{
                display: "flex",
                "flex-direction": "column",
                gap: "10px",
                opacity: qualitySupported.has(outputFormat()) ? 1 : 0.4,
              }}
            >
              <div
                style={{
                  display: "flex",
                  "justify-content": "space-between",
                  "align-items": "center",
                }}
              >
                <span
                  style={{
                    "font-size": "12px",
                    "font-weight": 600,
                    color: "var(--text-secondary)",
                    "letter-spacing": "0.5px",
                  }}
                >
                  画質
                </span>
                <Show
                  when={qualitySupported.has(outputFormat())}
                  fallback={
                    <span
                      style={{
                        "font-size": "11px",
                        color: "var(--text-muted)",
                      }}
                    >
                      {outputFormat().toUpperCase()} は画質設定非対応
                    </span>
                  }
                >
                  <span
                    class="mono"
                    style={{
                      "font-size": "12px",
                      color: "var(--accent)",
                    }}
                  >
                    {quality()}%
                  </span>
                </Show>
              </div>
              <input
                type="range"
                min="1"
                max="100"
                value={quality()}
                onInput={(e) => setQuality(parseInt(e.currentTarget.value))}
                disabled={!qualitySupported.has(outputFormat())}
                style={{
                  width: "100%",
                  "accent-color": "var(--accent)",
                  cursor: qualitySupported.has(outputFormat())
                    ? "pointer"
                    : "not-allowed",
                }}
              />
            </div>
          </GlassCard>

          {/* Action Area */}
          <div
            style={{
              display: "flex",
              "flex-direction": "column",
              gap: "16px",
            }}
          >
            <div
              style={{ display: "flex", gap: "12px", "align-items": "center" }}
            >
              <GradientButton
                onClick={handleConvert}
                disabled={files().length === 0 || isConverting()}
              >
                変換開始
              </GradientButton>
              <NeuButton disabled={!isConverting()}>キャンセル</NeuButton>
            </div>
            <Show when={isConverting() || progress() > 0}>
              <div
                style={{
                  display: "flex",
                  "flex-direction": "column",
                  gap: "8px",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    "justify-content": "space-between",
                  }}
                >
                  <span
                    style={{
                      "font-size": "12px",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {isConverting() ? "変換中..." : "完了"}
                  </span>
                  <span
                    class="mono"
                    style={{ "font-size": "12px", color: "var(--accent)" }}
                  >
                    {progress()}%
                  </span>
                </div>
                <ProgressBar percent={progress()} />
              </div>
            </Show>
          </div>
        </div>

        {/* Right Column */}
        <div
          style={{
            width: "320px",
            display: "flex",
            "flex-direction": "column",
            gap: "20px",
            "min-height": 0,
          }}
        >
          <GlassCard
            title={`選択ファイル (${files().length})`}
            style={{ flex: "1", overflow: "auto", "min-height": "0" }}
          >
            <Show
              when={files().length > 0}
              fallback={
                <div
                  style={{
                    flex: 1,
                    display: "flex",
                    "flex-direction": "column",
                    "align-items": "center",
                    "justify-content": "center",
                    gap: "8px",
                    padding: "32px 0",
                  }}
                >
                  <svg
                    width="40"
                    height="40"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="var(--text-muted)"
                    stroke-width="2"
                  >
                    <path d="M21 3H3a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h18a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Z" />
                  </svg>
                  <span
                    style={{ "font-size": "13px", color: "var(--text-muted)" }}
                  >
                    ファイル未選択
                  </span>
                </div>
              }
            >
              <div
                style={{
                  display: "flex",
                  "flex-direction": "column",
                  gap: "8px",
                }}
              >
                <For each={files()}>
                  {(file, i) => (
                    <FileListItem
                      file={file}
                      iconType="image"
                      onRemove={() => removeFile(i())}
                    />
                  )}
                </For>
              </div>
            </Show>
          </GlassCard>

          <Show when={results().length > 0}>
            <GlassCard
              title={`出力 (${results().length})`}
              style={{ "max-height": "280px", overflow: "auto" }}
            >
              <div
                style={{
                  display: "flex",
                  "flex-direction": "column",
                  gap: "8px",
                }}
              >
                <For each={results()}>
                  {(res) => (
                    <div
                      style={{
                        display: "flex",
                        gap: "10px",
                        "align-items": "center",
                        padding: "8px 12px",
                        background: "var(--bg-glass)",
                        border: "1px solid var(--glass-border)",
                        "border-radius": "var(--radius-sm)",
                      }}
                    >
                      <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="var(--success)"
                        stroke-width="2"
                        style={{ "flex-shrink": 0 }}
                      >
                        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                        <path d="m9 11 3 3L22 4" />
                      </svg>
                      <div
                        style={{
                          flex: 1,
                          "min-width": 0,
                          display: "flex",
                          "flex-direction": "column",
                          gap: "2px",
                        }}
                      >
                        <span
                          style={{
                            "font-size": "12px",
                            "font-weight": 500,
                            overflow: "hidden",
                            "text-overflow": "ellipsis",
                            "white-space": "nowrap",
                          }}
                        >
                          {res.output_path.split("/").pop()}
                        </span>
                        <span
                          class="mono"
                          style={{
                            "font-size": "10px",
                            color: "var(--success)",
                          }}
                        >
                          {formatFileSize(res.original_size)} →{" "}
                          {formatFileSize(res.output_size)}
                        </span>
                      </div>
                      <button
                        style={{
                          background: "none",
                          border: "none",
                          cursor: "pointer",
                          padding: "4px",
                          display: "flex",
                          color: "var(--accent)",
                        }}
                        onClick={() => revealItemInDir(res.output_path)}
                        title="出力先を開く"
                      >
                        <svg
                          width="14"
                          height="14"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2"
                        >
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

      <PixelLoader
        visible={isConverting()}
        label="変換中..."
        progress={progress()}
      />
    </>
  );
}
