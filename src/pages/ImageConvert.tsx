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
import ResultList, { type ConvertError } from "../components/ui/ResultList";
import Footer, { batchStatus } from "../components/ui/Footer";
import Field, { NoteText, ValueBadge } from "../components/ui/Field";
import Select from "../components/ui/Select";
import { DimensionRow } from "../components/ui/Inputs";
import Checkbox from "../components/ui/Checkbox";
import Slider from "../components/ui/Slider";
import PathPicker from "../components/ui/PathPicker";
import PixelLoader from "../components/ui/PixelLoader";
import { useFileDrop, pickFiles } from "../components/ui/useFileDrop";
import { useSelection } from "../components/ui/useSelection";
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

const columns: Column<FileInfo>[] = [
  thumbColumn("image"),
  nameColumn(),
  { header: "画像サイズ", width: "92px", nowrap: true, render: (f) => f.dimensions },
  { header: "ファイルサイズ", width: "96px", nowrap: true, render: (f) => f.size },
];

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
  const [errors, setErrors] = createSignal<ConvertError[]>([]);

  const selection = useSelection(files, setFiles);

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

  const isDragOver = useFileDrop(handleFilesSelected);

  // Yield to the browser so Solid.js can render pending state changes
  const yieldToUI = () =>
    new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));

  const handleConvert = async () => {
    const fileList = [...files()];
    if (fileList.length === 0) return;

    setIsConverting(true);
    setProgress(0);
    setResults([]);
    setErrors([]);

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
        const message = e instanceof Error ? e.message : String(e);
        console.error(`Conversion failed for ${file.name}:`, e);
        setErrors((prev) => [
          ...prev,
          { name: file.name, path: file.path, message },
        ]);
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

  const status = () =>
    batchStatus({
      fileCount: files().length,
      isConverting: isConverting(),
      progress: progress(),
      resultCount: results().length,
      errorCount: errors().length,
    });

  const canQuality = () => qualitySupported.has(outputFormat());

  return (
    <>
      <PageShell
        title="画像変換"
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
              <Select value={outputFormat()} onChange={setOutputFormat} options={imageFormats} />
            </Field>
            <Field label="リサイズ（任意）">
              <DimensionRow
                width={resizeWidth()}
                height={resizeHeight()}
                onWidth={setResizeWidth}
                onHeight={setResizeHeight}
              />
              <Checkbox
                checked={keepAspect()}
                onChange={() => setKeepAspect((v) => !v)}
                label="縦横比を維持"
              />
            </Field>
            <Field
              label="品質"
              dimmed={!canQuality()}
              aside={
                canQuality() ? (
                  <ValueBadge>{quality()}%</ValueBadge>
                ) : (
                  <NoteText>{outputFormat().toUpperCase()} は画質設定非対応</NoteText>
                )
              }
            >
              <Slider value={quality()} onChange={setQuality} disabled={!canQuality()} />
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
            ctaDisabled={files().length === 0 || isConverting()}
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
            icon: "image",
            label: "ここにファイルをドロップまたはクリックして選択",
            hint: "JPEG, PNG, GIF, WebP, BMP, HEIF",
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
      <PixelLoader visible={isConverting()} label="変換中..." progress={progress()} />
    </>
  );
}
