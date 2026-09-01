import { createReadStream, promises as fs } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { request as proxyRequest } from "node:http";
import { request as proxyRequestTls } from "node:https";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "dist");
const port = Number.parseInt(process.env.PORT ?? "8080", 10);
const upstream = new URL(process.env.API_UPSTREAM ?? "http://api:3000");
const proxy = upstream.protocol === "https:" ? proxyRequestTls : proxyRequest;

function contentType(path) {
  return {
    ".css": "text/css; charset=utf-8",
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".svg": "image/svg+xml",
    ".png": "image/png",
    ".woff2": "font/woff2",
  }[extname(path)] ?? "application/octet-stream";
}

function proxyApi(req, res) {
  const request = proxy(
    {
      hostname: upstream.hostname,
      port: upstream.port,
      path: req.url,
      method: req.method,
      headers: { ...req.headers, host: upstream.host },
    },
    (upstreamResponse) => {
      res.writeHead(upstreamResponse.statusCode ?? 502, upstreamResponse.headers);
      upstreamResponse.pipe(res);
    },
  );
  request.on("error", () => {
    if (!res.headersSent) res.writeHead(502, { "content-type": "text/plain" });
    res.end("API upstream unavailable\n");
  });
  req.pipe(request);
}

async function serveStatic(req, res) {
  let requested;
  try {
    requested = decodeURIComponent((req.url ?? "/").split("?", 1)[0] ?? "/");
  } catch {
    res.writeHead(400, { "content-type": "text/plain; charset=utf-8" });
    res.end("invalid URL\n");
    return;
  }
  const candidate = normalize(join(root, requested === "/" ? "index.html" : requested));
  const safeRoot = `${root}${sep}`;
  const path = candidate.startsWith(safeRoot) ? candidate : join(root, "index.html");
  try {
    const stat = await fs.stat(path);
    if (!stat.isFile()) throw new Error("not a file");
    res.writeHead(200, { "content-type": contentType(path), "cache-control": "no-cache" });
    createReadStream(path).pipe(res);
  } catch {
    const fallback = join(root, "index.html");
    res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    createReadStream(fallback).pipe(res);
  }
}

createServer((req, res) => {
  if ((req.url ?? "/").startsWith("/api/")) return proxyApi(req, res);
  return serveStatic(req, res);
}).listen(port, "0.0.0.0", () => {
  console.log(`frontend listening on 0.0.0.0:${port}; API upstream ${upstream.origin}`);
});
