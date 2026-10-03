import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// 本文件由 esbuild 在 Node 环境执行；不引入 @types/node，仅借声明读取构建期变量。
declare const process: { env: Record<string, string | undefined> };

// 开发时通过 proxy 把 /api（含 WebSocket）转发到 sen serve 的默认端口；
// 生产构建产物被 sen-server / 桌面端以静态资源方式托管（同源）。
// base 默认 "/"；GitHub Pages 等子路径部署用 VITE_BASE 覆盖（如 /SenAgent/）。
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: process.env.VITE_BASE || "/",
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8642",
        // 保持同源：不改写 Host（否则后端 Origin 校验会因 Host 变为目标地址而误拒）
        changeOrigin: false,
        ws: true,
      },
    },
  },
  build: {
    outDir: "dist",
    sourcemap: false,
  },
});
