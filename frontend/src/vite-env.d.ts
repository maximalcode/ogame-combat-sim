// Environment type declarations for Vite's `import.meta.env`.
//
// Vite exposes `import.meta.env.VITE_*` at build time; declaring the shape here
// keeps `tsc` strict about typos and lets the config module read the API base
// URL without a cast.

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL?: string;
  // PROTOTYPE — throwaway branch only: the switcher gates on dev builds.
  readonly DEV: boolean;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
