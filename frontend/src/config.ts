// API base URL configuration.
//
// combat-api's port is environment-driven (PORT env, default 3000), so the
// frontend cannot assume localhost:3000. The base URL is taken from
// `VITE_API_BASE_URL` at build time; when it is empty or unset the client
// uses same-origin requests, which the Vite dev server proxies to
// http://localhost:3000 (see vite.config.ts). That keeps a local dev workflow
// zero-config while still allowing a deployed frontend to point at any API.

const raw = import.meta.env.VITE_API_BASE_URL;

// Trailing slashes are stripped by walking backwards rather than with a `/+$/`
// regex: the regex form backtracks super-linearly on a long run of slashes, and
// a base URL is attacker-influenced often enough not to hand it that shape.
function withoutTrailingSlashes(value: string): string {
  let end = value.length;
  while (end > 0 && value[end - 1] === "/") end -= 1;
  return value.slice(0, end);
}

/** The configured API base URL, with no trailing slash. Empty means same-origin. */
export const API_BASE_URL: string = withoutTrailingSlashes((raw ?? "").trim());

/** True when the client should call the API same-origin (dev proxy or co-hosted). */
export const isSameOrigin: boolean = API_BASE_URL.length === 0;

/** Build a full URL for a given API path. */
export function apiUrl(path: string): string {
  if (isSameOrigin) return path;
  return `${API_BASE_URL}${path}`;
}
