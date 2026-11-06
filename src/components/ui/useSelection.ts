import { createSignal, createEffect, createMemo, type Accessor, type Setter } from "solid-js";

export function useSelection<T extends { path: string }>(
  files: Accessor<T[]>,
  setFiles: Setter<T[]>,
) {
  const [selected, setSelected] = createSignal<Set<string>>(new Set());

  // Conversion also removes files, so selection must follow the file list
  createEffect(() => {
    const present = new Set(files().map((f) => f.path));
    setSelected((prev) => {
      const next = new Set([...prev].filter((p) => present.has(p)));
      return next.size === prev.size ? prev : next;
    });
  });

  const count = createMemo(() => selected().size);
  const allSelected = createMemo(
    () => files().length > 0 && files().every((f) => selected().has(f.path)),
  );
  const partial = createMemo(() => count() > 0 && !allSelected());

  const isSelected = (path: string) => selected().has(path);

  const toggle = (path: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (!next.delete(path)) next.add(path);
      return next;
    });
  };

  const toggleAll = () => {
    setSelected(allSelected() ? new Set<string>() : new Set(files().map((f) => f.path)));
  };

  const removeSelected = () => {
    const sel = selected();
    setFiles((prev) => prev.filter((f) => !sel.has(f.path)));
  };

  return { count, allSelected, partial, isSelected, toggle, toggleAll, removeSelected };
}

export type Selection = ReturnType<typeof useSelection>;
