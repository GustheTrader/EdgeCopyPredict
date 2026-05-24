import fs from "node:fs";
import fsp from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as esbuild from "esbuild";

export const rootDir = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
export const srcDir = path.join(rootDir, "src");
export const entryPoint = path.join(srcDir, "main.tsx");
export const distDir = path.join(rootDir, "dist");
export const assetsDir = path.join(distDir, "assets");

function resolveSourceImport(importPath) {
  const basePath = path.join(srcDir, importPath.slice(2));
  const candidates = [
    basePath,
    `${basePath}.ts`,
    `${basePath}.tsx`,
    `${basePath}.js`,
    `${basePath}.jsx`,
    path.join(basePath, "index.ts"),
    path.join(basePath, "index.tsx"),
  ];

  return candidates.find((candidate) => fs.existsSync(candidate));
}

export function sourceAliasPlugin() {
  return {
    name: "source-alias",
    setup(build) {
      build.onResolve({ filter: /^@\// }, (args) => {
        const resolved = resolveSourceImport(args.path);
        if (!resolved) {
          return {
            errors: [{ text: `Could not resolve ${args.path}` }],
          };
        }
        return { path: resolved };
      });
    },
  };
}

export async function processCss(outDir = distDir) {
  const cssPath = path.join(outDir, "assets", "app.css");
  if (!fs.existsSync(cssPath)) return false;

  const [{ default: postcss }, { default: tailwindcss }, { default: autoprefixer }] =
    await Promise.all([
      import(path.join(rootDir, "node_modules/postcss/lib/postcss.js")),
      import(path.join(rootDir, "node_modules/tailwindcss/lib/index.js")),
      import(path.join(rootDir, "node_modules/autoprefixer/lib/autoprefixer.js")),
    ]);

  const css = await fsp.readFile(cssPath, "utf8");
  const result = await postcss([tailwindcss, autoprefixer]).process(css, {
    from: path.join(rootDir, "src", "styles.css"),
    to: cssPath,
  });
  await fsp.writeFile(cssPath, result.css);
  return true;
}

const TAILWIND_CDN_BLOCK = `    <script>
      tailwind = { config: { darkMode: 'class', theme: { extend: { colors: { surface: { DEFAULT: '#0f1117', card: '#1a1d27', border: '#2a2d3a' }, accent: { green: '#22c55e', red: '#ef4444', yellow: '#eab308', blue: '#3b82f6' } }, fontFamily: { mono: ['JetBrains Mono', 'ui-monospace', 'monospace'] } } } } };
    </script>
    <script src="https://cdn.tailwindcss.com"></script>
    <style type="text/tailwindcss">
      @layer base {
        * { box-sizing: border-box; }
        html { font-family: "Inter", "ui-sans-serif", system-ui, sans-serif; font-size: 14px; }
        ::-webkit-scrollbar { width: 6px; height: 6px; }
        ::-webkit-scrollbar-track { background: transparent; }
        ::-webkit-scrollbar-thumb { background: #2a2d3a; border-radius: 3px; }
      }
    </style>
    <style>html,body{background:#0f1117;color:#fff;margin:0}</style>`;

export async function writeIndex(outDir = distDir) {
  const source = await fsp.readFile(path.join(rootDir, "index.html"), "utf8");
  const isDev = outDir !== distDir;
  const headInsert = isDev ? `${TAILWIND_CDN_BLOCK}\n  </head>` : `  </head>`;
  const html = source
    .replace("  </head>", headInsert)
    .replace(
      '    <script type="module" src="/src/main.tsx"></script>',
      `    <script type="module" src="/assets/app.js"></script>`,
    );
  await fsp.writeFile(path.join(outDir, "index.html"), html);
}

export async function buildApp({ outDir = distDir, sourcemap = false } = {}) {
  console.log("cleaning output directory");
  await fsp.rm(outDir, { recursive: true, force: true });
  await fsp.mkdir(path.join(outDir, "assets"), { recursive: true });

  console.log("bundling dashboard");
  await esbuild.build({
    entryPoints: [entryPoint],
    bundle: true,
    format: "esm",
    outfile: path.join(outDir, "assets", "app.js"),
    sourcemap,
    minify: !sourcemap,
    target: "es2020",
    jsx: "automatic",
    loader: {
      ".css": "css",
    },
    plugins: [sourceAliasPlugin()],
    logLevel: "silent",
  });

  console.log("processing styles");
  await processCss(outDir);
  console.log("writing index");
  await writeIndex(outDir);
}

export function proxyApiRequest(req, res) {
  const target = new URL(req.url ?? "/", "http://127.0.0.1:8080");
  const upstream = http.request(
    target,
    {
      method: req.method,
      headers: {
        ...req.headers,
        host: "127.0.0.1:8080",
      },
    },
    (upstreamRes) => {
      res.writeHead(upstreamRes.statusCode ?? 502, upstreamRes.headers);
      upstreamRes.pipe(res);
    },
  );

  upstream.on("error", (error) => {
    res.writeHead(502, { "content-type": "application/json" });
    res.end(JSON.stringify({ error: error.message }));
  });

  req.pipe(upstream);
}

if (fileURLToPath(import.meta.url) === path.resolve(process.argv[1] ?? "")) {
  await buildApp();
  console.log(`built ${path.relative(rootDir, distDir)}`);
}
