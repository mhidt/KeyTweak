import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        toast: resolve(__dirname, "toast.html"),
      },
    },
  },
  server: {
    strictPort: true,
  },
});
