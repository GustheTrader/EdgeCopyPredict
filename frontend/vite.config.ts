import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "VITE_");
  const port = parseInt(env.VITE_PORT ?? "3000", 10);
  const apiBase = env.VITE_API_BASE_URL ?? "http://localhost:8080";

  return {
    plugins: [react()],
    resolve: {
      alias: {
        "@": path.resolve(__dirname, "./src"),
      },
    },
    optimizeDeps: {
      include: [
        "react",
        "react-dom",
        "react-router-dom",
        "@tanstack/react-query",
        "zustand",
        "lightweight-charts",
        "clsx",
      ],
    },
    server: {
      port,
      proxy: {
        "/api": {
          target: apiBase,
          changeOrigin: true,
        },
        "/sse": {
          target: apiBase,
          changeOrigin: true,
          // SSE requires streaming — disable http-proxy's response buffering
          configure: (proxy) => {
            proxy.on("proxyRes", (proxyRes) => {
              proxyRes.headers["cache-control"] = "no-cache";
              proxyRes.headers["x-accel-buffering"] = "no";
            });
            proxy.on("error", (_err, _req, res: any) => {
              if (res && !res.headersSent) {
                res.writeHead(502);
                res.end();
              }
            });
          },
        },
      },
    },
  };
});
