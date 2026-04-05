import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vitest/config";
import { sveltekit } from "@sveltejs/kit/vite";
import type { IncomingMessage } from "node:http";

const isBuild = process.env.npm_lifecycle_event === "build";
const DEV_BACKEND_ORIGIN = "http://127.0.0.1:8245";
const DEV_AUTH_POST_PATHS = new Set(["/auth/login", "/auth/setup"]);

function readRequestBody(req: IncomingMessage): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    req.on("data", (chunk: Buffer | string) => {
      chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
    });
    req.on("end", () => resolve(Buffer.concat(chunks)));
    req.on("error", reject);
  });
}

// Rollup circular-dependency warnings from d3 (pulled in by @xyflow/system) are non-actionable
// and clutter production build logs. Filter them only for `npm run build`.
if (isBuild) {
  const originalWarn = console.warn;
  console.warn = (...args: unknown[]) => {
    const message = args
      .map((a) => {
        if (typeof a === "string") return a;
        if (a && typeof a === "object" && "message" in a) {
          return String((a as any).message);
        }
        if (a && typeof (a as any)?.toString === "function")
          return String((a as any).toString());
        return "";
      })
      .join(" ");

    // Example: "Circular dependency: node_modules/d3-selection/src/selection/index.js -> ..."
    if (
      message.startsWith("Circular dependency:") &&
      message.includes("node_modules/d3-")
    ) {
      return;
    }

    originalWarn(...args);
  };
}

export default defineConfig({
  plugins: [
    tailwindcss(),
    sveltekit(),
    {
      name: "dev-auth-post-proxy",
      apply: "serve",
      configureServer(server) {
        server.middlewares.use(async (req, res, next) => {
          const method = req.method ?? "GET";
          const url = req.url ?? "/";
          const pathname = url.split("?")[0] ?? url;

          if (method !== "POST" || !DEV_AUTH_POST_PATHS.has(pathname)) {
            return next();
          }

          try {
            const body = await readRequestBody(req);
            const headers = new Headers();
            const accept = req.headers.accept;
            const contentType = req.headers["content-type"];
            const cookie = req.headers.cookie;

            if (accept) headers.set("accept", accept);
            if (contentType) headers.set("content-type", contentType);
            if (cookie) headers.set("cookie", cookie);

            const upstream = await fetch(`${DEV_BACKEND_ORIGIN}${url}`, {
              method: "POST",
              headers,
              body,
            });

            res.statusCode = upstream.status;

            const responseHeaders = new Headers(upstream.headers);
            responseHeaders.delete("content-encoding");
            responseHeaders.delete("transfer-encoding");
            responseHeaders.delete("connection");

            for (const [key, value] of responseHeaders) {
              if (key.toLowerCase() !== "set-cookie") {
                res.setHeader(key, value);
              }
            }

            const setCookie = upstream.headers.getSetCookie?.() ?? [];
            if (setCookie.length > 0) {
              res.setHeader("set-cookie", setCookie);
            }

            const payload = Buffer.from(await upstream.arrayBuffer());
            res.end(payload);
          } catch (error) {
            next(error as Error);
          }
        });
      },
    },
  ],
  // This project ships with a couple of known dependency-level warnings (monaco/d3/xyflow).
  // Keep build logs actionable by suppressing warnings while still failing on real errors.
  logLevel: isBuild ? "error" : "info",
  build: {
    // Monaco produces very large chunks; raising the threshold avoids noisy non-actionable warnings.
    chunkSizeWarningLimit: 5000,
    rollupOptions: {
      // Keep build output focused on project issues; most warnings below come from deps.
      onwarn(warning, warn) {
        const code =
          typeof warning === "object" ? (warning as any)?.code : undefined;
        const message =
          typeof warning === "string"
            ? warning
            : ((warning as any)?.message ??
              (warning as any)?.toString?.() ??
              "");

        // @xyflow build noise: dependency imports an internal hook it doesn't end up using.
        if (
          message.includes("handleConnectionChange") &&
          message.includes("@xyflow")
        )
          return;

        // d3-* circular dependency chains; not a runtime error.
        if (code === "CIRCULAR_DEPENDENCY") return;
        if (message.includes("Circular dependency")) return;

        warn(warning);
      },
      output: {
        // Manual chunking reduces the chance that one dependency dominates a single bundle.
        manualChunks(id) {
          if (!id.includes("node_modules")) return undefined;

          if (id.includes("monaco-editor")) return "monaco-editor";
          if (id.includes("@xyflow")) return "xyflow";

          // Keep d3 in its own chunk group to avoid mixing with other large UI deps.
          if (id.includes("/d3-")) return "d3";

          return undefined;
        },
      },
    },
  },
  server: {
    host: true,
    allowedHosts: true,
    cors: false,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8245",
        changeOrigin: true,
      },
      "/ws": {
        target: "ws://127.0.0.1:8245",
        ws: true,
        changeOrigin: true,
      },
    },
    fs: {
      strict: false,
    },
  },
  test: {
    expect: { requireAssertions: true },
    projects: [
      {
        extends: "./vite.config.ts",
        test: {
          name: "server",
          environment: "node",
          include: [
            "src/**/*.{test,spec}.{js,ts}",
            "demo/**/*.{test,spec}.{js,ts}",
          ],
          exclude: ["src/**/*.svelte.{test,spec}.{js,ts}"],
        },
      },
    ],
  },
});
