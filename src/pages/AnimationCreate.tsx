import { createSignal, For, Show } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import DropZone from "../components/ui/DropZone";
import GlassCard from "../components/ui/GlassCard";
import NeuSelect from "../components/ui/NeuSelect";
import GradientButton from "../components/ui/GradientButton";
import NeuButton from "../components/ui/NeuButton";
import PixelLoader from "../components/ui/PixelLoader";
import FileListItem, { type FileInfo } from "../components/ui/FileListItem";
import {
  getImageInfo,
  createAnimation,
  formatFileSize,
  type CreateAnimationResult,
} from "../lib/tauri";

const animFormats = [
  { value: "gif", label: "GIF" },
  { value: "awebp", label: "Animated WebP" },
  { value: "mp4", label: "MP4" },
];

export default function AnimationCreate() {
  const [files, setFiles] = createSignal<FileInfo[]>([]);
  const [outputFormat, setOutputFormat] = createSignal("gif");
  const [outputDir, setOutputDir] = createSignal("~/Downloads");
  const [outputName, setOutputName] = createSignal("animation");
  const [fps, setFps] = createSignal(24);
  const [isCreating, setIsCreating] = createSignal(false);
  const [result, setResult] = createSignal<CreateAnimationResult | null>(null);

  const handleFilesSelected = async (paths: string[]) => {
    // Sort paths naturally for sequential images
    const sorted = [...paths].sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));
    for (const path of sorted) {
      try {
        const info = await getImageInfo(path);
        setFiles((prev) => [
          ...prev,
          {
            path: info.path,
            name: info.name,
            size: formatFileSize(info.size),
            dimensions: `${info.width}×${info.height}`,
            thumbnail: info.thumbnail,
          },
        ]);
      } catch (e) {
        console.error("Failed to get image info:", e);
      }
    }
  };

  const removeFile = (index: number) => {
    setFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const moveFile = (from: number, to: number) => {
    setFiles((prev) => {
      const arr = [...prev];
      const [item] = arr.splice(from, 1);
      arr.splice(to, 0, item);
      return arr;
    });
  };

  const yieldToUI = () =>
    new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));

  const handleCreate = async () => {
    if (files().length === 0) return;

    setIsCreating(true);
    setResult(null);
    await yieldToUI();

    try {
      const res = await createAnimation({
        image_paths: files().map((f) => f.path),
        output_format: outputFormat(),
        output_dir: outputDir(),
        output_name: outputName(),
        fps: fps(),
      });
      setResult(res);
      setFiles([]);
    } catch (e) {
      console.error("Animation creation failed:", e);
    } finally {
      setIsCreating(false);
    }
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
      <header style={{ display: "flex", "justify-content": "space-between", "align-items": "center" }}>
        <h1 style={{ "font-size": "24px", "font-weight": 600, "letter-spacing": "-0.5px" }}>
          アニメーション作成
        </h1>
        <span style={{ "font-size": "13px", color: "var(--text-secondary)" }}>
          連番画像からアニメーションを生成
        </span>
      </header>

      <div style={{ flex: 1, display: "flex", gap: "24px", "min-height": 0 }}>
        {/* Left Column */}
        <div style={{ flex: 1, display: "flex", "flex-direction": "column", gap: "20px" }}>
          <DropZone
            icon="images"
            label="連番画像をドロップまたはクリックして選択"
            hint="PNG, JPEG, BMP（複数選択可）"
            onFilesSelected={handleFilesSelected}
          />

          <GlassCard title="出力設定">
            {/* Output Format */}
            <div style={{ display: "flex", gap: "12px", "align-items": "center" }}>
              <span style={{ "font-size": "12px", "font-weight": 500, color: "var(--text-secondary)", "letter-spacing": "0.5px" }}>
                出力形式
              </span>
              <NeuSelect value={outputFormat()} onChange={setOutputFormat} options={animFormats} style={{ width: "180px" }} />
            </div>

            {/* Output name */}
            <div style={{ display: "flex", gap: "12px", "align-items": "center" }}>
              <span style={{ "font-size": "12px", "font-weight": 500, color: "var(--text-secondary)", "letter-spacing": "0.5px" }}>
                ファイル名
              </span>
              <input
                class="glass"
                type="text"
                value={outputName()}
                onInput={(e) => setOutputName(e.currentTarget.value)}
                style={{
                  flex: 1, padding: "10px 14px", "border-radius": "var(--radius-md)", "font-size": "13px",
                  color: "var(--text-primary)", background: "var(--bg-glass)", border: "1px solid var(--glass-border)", "font-family": "var(--font-ui)",
                }}
              />
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

            {/* FPS */}
            <div style={{ display: "flex", gap: "12px", "align-items": "center" }}>
              <span style={{ "font-size": "12px", "font-weight": 500, color: "var(--text-secondary)", "letter-spacing": "0.5px" }}>
                FPS
              </span>
              <input
                class="glass"
                type="number"
                min="1"
                max="60"
                value={fps()}
                onInput={(e) => setFps(parseInt(e.currentTarget.value) || 24)}
                style={{
                  width: "80px", padding: "10px 14px", "border-radius": "var(--radius-md)", "font-size": "13px",
                  color: "var(--text-primary)", background: "var(--bg-glass)", border: "1px solid var(--glass-border)", "font-family": "var(--font-ui)",
                }}
              />
              <span style={{ "font-size": "11px", color: "var(--text-muted)" }}>
                フレーム/秒
              </span>
            </div>
          </GlassCard>

          {/* Action Area */}
          <div style={{ display: "flex", gap: "12px", "align-items": "center" }}>
            <GradientButton onClick={handleCreate} disabled={files().length === 0 || isCreating()}>
              生成開始
            </GradientButton>
            <NeuButton disabled={!isCreating()}>キャンセル</NeuButton>
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
                    <path d="M18 22H4a2 2 0 0 1-2-2V6 M21 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2Z" />
                  </svg>
                  <span style={{ "font-size": "13px", color: "var(--text-muted)" }}>画像が未選択です</span>
                  <span style={{ "font-size": "11px", color: "var(--text-muted)" }}>左のエリアから追加してください</span>
                </div>
              }
            >
              <div style={{ display: "flex", "flex-direction": "column", gap: "4px" }}>
                <For each={files()}>
                  {(file, i) => (
                    <div style={{ display: "flex", "align-items": "center", gap: "4px" }}>
                      <span class="mono" style={{ "font-size": "10px", color: "var(--text-muted)", width: "24px", "text-align": "right", "flex-shrink": 0 }}>
                        {i() + 1}
                      </span>
                      <div style={{ flex: 1 }}>
                        <FileListItem file={file} iconType="image" onRemove={() => removeFile(i())} />
                      </div>
                      <div style={{ display: "flex", "flex-direction": "column", gap: "2px" }}>
                        <button
                          style={{ background: "none", border: "none", cursor: i() > 0 ? "pointer" : "default", padding: "2px", display: "flex", color: i() > 0 ? "var(--text-muted)" : "transparent" }}
                          onClick={() => i() > 0 && moveFile(i(), i() - 1)}
                        >
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m18 15-6-6-6 6" /></svg>
                        </button>
                        <button
                          style={{ background: "none", border: "none", cursor: i() < files().length - 1 ? "pointer" : "default", padding: "2px", display: "flex", color: i() < files().length - 1 ? "var(--text-muted)" : "transparent" }}
                          onClick={() => i() < files().length - 1 && moveFile(i(), i() + 1)}
                        >
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m6 9 6 6 6-6" /></svg>
                        </button>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </GlassCard>

          <Show when={result()}>
            {(res) => (
              <GlassCard title="出力">
                <div style={{ display: "flex", gap: "10px", "align-items": "center" }}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2" style={{ "flex-shrink": 0 }}>
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <path d="m9 11 3 3L22 4" />
                  </svg>
                  <div style={{ flex: 1, "min-width": 0, display: "flex", "flex-direction": "column", gap: "2px" }}>
                    <span style={{ "font-size": "12px", "font-weight": 500, overflow: "hidden", "text-overflow": "ellipsis", "white-space": "nowrap" }}>
                      {res().output_path.split("/").pop()}
                    </span>
                    <span class="mono" style={{ "font-size": "10px", color: "var(--success)" }}>
                      {res().frame_count} フレーム · {formatFileSize(res().output_size)}
                    </span>
                  </div>
                </div>
                <div style={{ display: "flex", gap: "8px", "align-items": "center" }}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="2">
                    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                  </svg>
                  <span class="mono" style={{ "font-size": "11px", color: "var(--text-secondary)", overflow: "hidden", "text-overflow": "ellipsis", "white-space": "nowrap" }}>
                    {res().output_path}
                  </span>
                </div>
                <NeuButton onClick={() => revealItemInDir(res().output_path)}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2">
                    <path d="M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                  </svg>
                  出力先を開く
                </NeuButton>
              </GlassCard>
            )}
          </Show>
        </div>
      </div>

      <PixelLoader visible={isCreating()} label="アニメーション生成中..." />
    </>
  );
}
