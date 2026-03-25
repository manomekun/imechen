import { Router, Route, Navigate } from "@solidjs/router";
import AppLayout from "./components/layout/AppLayout";
import ImageConvert from "./pages/ImageConvert";
import VideoConvert from "./pages/VideoConvert";
import AnimationCreate from "./pages/AnimationCreate";
import Settings from "./pages/Settings";
import { useTheme } from "./stores/theme";

export default function App() {
  useTheme();

  return (
    <Router root={AppLayout}>
      <Route path="/image" component={ImageConvert} />
      <Route path="/video" component={VideoConvert} />
      <Route path="/animation" component={AnimationCreate} />
      <Route path="/settings" component={Settings} />
      <Route path="*" component={() => <Navigate href="/image" />} />
    </Router>
  );
}
