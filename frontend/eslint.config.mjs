// Consumes the maxi-quality baseline. Add project-specific overrides below the
// spread — see docs/ADOPTION.md §2. Regenerate eslint.base.mjs with scripts/adopt.sh.
import base from './eslint.base.mjs';

export default [
  ...base,
  { languageOptions: { parserOptions: { tsconfigRootDir: import.meta.dirname } } },
];
