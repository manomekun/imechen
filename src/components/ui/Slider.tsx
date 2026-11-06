import styles from "./Slider.module.css";

interface SliderProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  disabled?: boolean;
}

export default function Slider(props: SliderProps) {
  const min = () => props.min ?? 1;
  const max = () => props.max ?? 100;
  const fill = () => ((props.value - min()) / (max() - min())) * 100;

  return (
    <input
      type="range"
      class={styles.slider}
      min={min()}
      max={max()}
      value={props.value}
      disabled={props.disabled}
      style={{ "--fill": `${fill()}%` }}
      onInput={(e) => props.onChange(parseInt(e.currentTarget.value))}
    />
  );
}
