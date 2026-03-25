import { useTheme, type ThemeMode } from "../stores/theme";

export default function Settings() {
  const { themeMode, setThemeMode } = useTheme();

  const themes: { mode: ThemeMode; label: string; icon: string }[] = [
    { mode: "dark", label: "ダーク", icon: "🌙" },
    { mode: "light", label: "ライト", icon: "☀️" },
    { mode: "system", label: "システム", icon: "🖥️" },
  ];

  return (
    <>
      <header style={{ display: "flex", "justify-content": "space-between", "align-items": "center" }}>
        <h1 style={{ "font-size": "24px", "font-weight": 600 }}>設定</h1>
        <span style={{ "font-size": "13px", color: "var(--text-secondary)" }}>アプリケーションの設定</span>
      </header>
      <div style={{ display: "flex", "flex-direction": "column", gap: "24px", "max-width": "560px" }}>
        <div class="glass" style={{ "border-radius": "var(--radius-lg)", padding: "20px", display: "flex", "flex-direction": "column", gap: "16px" }}>
          <span style={{ "font-weight": 600, "font-size": "14px" }}>テーマ</span>
          <span style={{ "font-size": "13px", color: "var(--text-secondary)" }}>アプリの外観テーマを選択してください</span>
          <div style={{ display: "flex", gap: "12px" }}>
            {themes.map((t) => (
              <button
                onClick={() => setThemeMode(t.mode)}
                style={{
                  flex: 1,
                  padding: "16px",
                  display: "flex",
                  "flex-direction": "column",
                  "align-items": "center",
                  gap: "8px",
                  "border-radius": "var(--radius-md)",
                  background: "var(--bg-elevated)",
                  border: themeMode() === t.mode ? "2px solid var(--accent)" : "1px solid var(--glass-border)",
                  cursor: "pointer",
                  color: "inherit",
                }}
              >
                <span style={{ "font-size": "24px" }}>{t.icon}</span>
                <span style={{
                  "font-size": "13px",
                  "font-weight": themeMode() === t.mode ? 600 : 400,
                  color: themeMode() === t.mode ? "var(--text-primary)" : "var(--text-secondary)",
                }}>{t.label}</span>
              </button>
            ))}
          </div>
        </div>
        <div style={{ color: "var(--text-muted)", "font-size": "12px" }}>
          <span class="mono">imechen v0.1.0</span>
        </div>
      </div>
    </>
  );
}
