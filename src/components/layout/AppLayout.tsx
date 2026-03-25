import type { ParentProps } from "solid-js";
import Sidebar from "./Sidebar";

export default function AppLayout(props: ParentProps) {
  return (
    <div style={{ display: "flex", height: "100vh", overflow: "hidden" }}>
      <Sidebar />
      <main
        style={{
          flex: 1,
          overflow: "auto",
          padding: "32px 40px",
          display: "flex",
          "flex-direction": "column",
          gap: "24px",
        }}
      >
        {props.children}
      </main>
    </div>
  );
}
