import { invoke } from "@tauri-apps/api/core";

export interface ImageInfo {
  path: string;
  name: string;
  size: number;
  width: number;
  height: number;
  format: string;
  thumbnail: string;
}

export interface ConvertImageRequest {
  source_path: string;
  output_format: string;
  output_dir: string;
  quality?: number;
  resize_width?: number;
  resize_height?: number;
  keep_aspect_ratio?: boolean;
}

export interface ConvertImageResult {
  output_path: string;
  original_size: number;
  output_size: number;
  width: number;
  height: number;
}

export async function getImageInfo(path: string): Promise<ImageInfo> {
  return invoke("get_image_info", { path });
}

export async function convertImage(
  request: ConvertImageRequest,
): Promise<ConvertImageResult> {
  return invoke("convert_image", { request });
}

// === Video ===

export interface VideoInfo {
  path: string;
  name: string;
  size: number;
  width: number;
  height: number;
  duration_secs: number;
  format: string;
  codec: string;
}

export interface ConvertVideoRequest {
  source_path: string;
  output_format: string;
  output_dir: string;
  quality?: number;
  resize_width?: number;
  resize_height?: number;
}

export interface ConvertVideoResult {
  output_path: string;
  original_size: number;
  output_size: number;
}

export interface VideoProgress {
  percent: number;
  fps: number;
  speed: string;
}

export async function getVideoInfo(path: string): Promise<VideoInfo> {
  return invoke("get_video_info", { path });
}

export async function convertVideo(
  request: ConvertVideoRequest,
): Promise<ConvertVideoResult> {
  return invoke("convert_video", { request });
}

export async function ensureFfmpeg(): Promise<boolean> {
  return invoke("ensure_ffmpeg");
}

// === Animation ===

export interface CreateAnimationRequest {
  image_paths: string[];
  output_format: string;
  output_dir: string;
  output_name: string;
  fps: number;
}

export interface CreateAnimationResult {
  output_path: string;
  frame_count: number;
  output_size: number;
}

export async function createAnimation(
  request: CreateAnimationRequest,
): Promise<CreateAnimationResult> {
  return invoke("create_animation", { request });
}

// === Utilities ===

export function formatDuration(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  if (h > 0) return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
