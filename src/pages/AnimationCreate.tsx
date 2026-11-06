import { createSignal } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import PageShell from "../components/ui/PageShell";
import Toolbar from "../components/ui/Toolbar";
import FileTable, {
  nameColumn,
  thumbColumn,
  type Column,
  type FileInfo,
} from "../components/ui/FileTable";
import ResultList from "../components/ui/ResultList";
import Footer from "../components/ui/Footer";
import Field from "../components/ui/Field";
import Select from "../components/ui/Select";
import { NumberInput, TextInput } from "../components/ui/Inputs";
import PathPicker from "../components/ui/PathPicker";
import PixelLoader from "../components/ui/PixelLoader";
import Icon from "../components/ui/Icon";
import { useFileDrop, pickFiles } from "../components/ui/useFileDrop";
import { useSelection } from "../components/ui/useSelection";
import {
  getImageInfo,
  createAnimation,
  formatFileSize,
  type CreateAnimationResult,
} from "../lib/tauri";
import styles from "./AnimationCreate.module.css";

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

  const selection = useSelection(files, setFiles);

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

  const isDragOver = useFileDrop(handleFilesSelected);

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

  const columns: Column<FileInfo>[] = [
    {
      header: "#",
      width: "24px",
      render: (_, i) => <span class={styles.index}>{i + 1}</span>,
    },
    thumbColumn("image"),
    nameColumn(),
    { header: "画像サイズ", width: "92px", nowrap: true, render: (f) => f.dimensions },
    { header: "ファイルサイズ", width: "96px", nowrap: true, render: (f) => f.size },
    {
      header: "",
      width: "28px",
      render: (_, i) => (
        <div class={styles.reorder}>
          <button
            type="button"
            class={styles.move}
            disabled={i === 0}
            onClick={() => moveFile(i, i - 1)}
            aria-label="上へ移動"
          >
            <Icon name="chevronUp" size={14} />
          </button>
          <button
            type="button"
            class={styles.move}
            disabled={i === files().length - 1}
            onClick={() => moveFile(i, i + 1)}
            aria-label="下へ移動"
          >
            <Icon name="chevronDown" size={14} />
          </button>
        </div>
      ),
    },
  ];

  return (
    <>
      <PageShell
        title="アニメーション作成"
        toolbar={
          <Toolbar
            onAdd={() => pickFiles(handleFilesSelected)}
            onDelete={selection.removeSelected}
            deleteDisabled={selection.count() === 0}
          />
        }
        panelTitle="出力設定"
        panel={
          <>
            <Field label="出力形式">
              <Select value={outputFormat()} onChange={setOutputFormat} options={animFormats} />
            </Field>
            <Field label="ファイル名">
              <TextInput value={outputName()} onInput={setOutputName} />
            </Field>
            <Field label="FPS">
              <div class={styles.fps}>
                <NumberInput
                  min={1}
                  max={60}
                  value={fps()}
                  onInput={(v) => setFps(parseInt(v) || 24)}
                  ariaLabel="FPS"
                />
                <span class={styles.suffix}>フレーム/秒</span>
              </div>
            </Field>
            <Field label="保存先">
              <PathPicker path={outputDir()} onSelect={selectOutputDir} />
            </Field>
          </>
        }
        footer={
          <Footer
            status={`${files().length} ファイル`}
            ctaLabel="生成する"
            ctaDisabled={files().length === 0 || isCreating()}
            onCta={handleCreate}
          />
        }
      >
        <FileTable
          files={files()}
          columns={columns}
          selection={selection}
          isDragOver={isDragOver()}
          empty={{
            icon: "images",
            label: "連番画像をドロップまたはクリックして選択",
            hint: "PNG, JPEG, BMP（複数選択可）",
          }}
          onPick={() => pickFiles(handleFilesSelected)}
        />
        <ResultList
          outputs={(() => {
            const res = result();
            if (!res) return [];
            return [
              {
                name: res.output_path.split("/").pop() ?? res.output_path,
                detail: `${res.frame_count} フレーム · ${formatFileSize(res.output_size)}`,
                path: res.output_path,
                fullPath: res.output_path,
              },
            ];
          })()}
        />
      </PageShell>
      <PixelLoader visible={isCreating()} label="アニメーション生成中..." />
    </>
  );
}
