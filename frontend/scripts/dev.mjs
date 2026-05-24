import fs from "node:fs";
import fsp from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import * as esbuild from "esbuild";
import {
  entryPoint,
  processCss,
  proxyApiRequest,
  rootDir,
  sourceAliasPlugin,
  writeIndex,
} from "./build.mjs";

const host = "0.0.0.0";
const port = Number(process.env.PORT ?? 5173);
const devDir = path.join(rootDir, ".dev");

function contentType(filePath) {
  if (filePath.endsWith(".html")) return "text/html; charset=utf-8";
  if (filePath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (filePath.endsWith(".css")) return "text/css; charset=utf-8";
  if (filePath.endsWith(".svg")) return "image/svg+xml";
  if (filePath.endsWith(".json")) return "application/json";
  return "application/octet-stream";
}

async function serveFile(res, filePath) {
  try {
    const data = await fsp.readFile(filePath);
    res.writeHead(200, { "content-type": contentType(filePath) });
    res.end(data);
  } catch {
    res.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    res.end("not found");
  }
}

async function prepareIndex() {
  await processCss(devDir);
  await writeIndex(devDir);
}

await fsp.rm(devDir, { recursive: true, force: true });
await fsp.mkdir(path.join(devDir, "assets"), { recursive: true });

const ctx = await esbuild.context({
  entryPoints: [entryPoint],
  bundle: true,
  format: "esm",
  outfile: path.join(devDir, "assets", "app.js"),
  sourcemap: true,
  target: "es2020",
  jsx: "automatic",
  loader: {
    ".css": "css",
  },
  plugins: [
    sourceAliasPlugin(),
    {
      name: "post-build-css",
      setup(build) {
        build.onEnd(async (result) => {
          if (result.errors.length === 0) {
            await prepareIndex();
          }
        });
      },
    },
  ],
});

await ctx.watch();
await ctx.rebuild();

const server = http.createServer(async (req, res) => {
  const requestUrl = req.url ?? "/";
  if (requestUrl.startsWith("/api/")) {
    proxyApiRequest(req, res);
    return;
  }

  if (requestUrl.startsWith("/ws")) {
    res.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    res.end("websocket proxy is not available in esbuild dev mode");
    return;
  }

  const url = new URL(requestUrl, `http://${host}:${port}`);
  const pathname = decodeURIComponent(url.pathname);
  const filePath =
    pathname === "/"
      ? path.join(devDir, "index.html")
      : path.join(devDir, pathname);

  if (filePath.startsWith(devDir) && fs.existsSync(filePath)) {
    await serveFile(res, filePath);
    return;
  }

  await serveFile(res, path.join(devDir, "index.html"));
});

server.listen(port, host, () => {
  console.log(`dashboard dev server: http://${host}:${port}`);
});

async function shutdown() {
  server.close();
  await ctx.dispose();
  process.exit(0);
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
