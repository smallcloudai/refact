import { defineConfig } from "eslint/config";
// import globals from "globals";
// import js from "@eslint/js";
// import tseslint from "typescript-eslint";
import playwright from "eslint-plugin-playwright";

export default defineConfig([
  // start of: eslint init defaults
  // { files: ["**/*.{js,mjs,cjs,ts}"] },
  // {
  //   files: ["**/*.{js,mjs,cjs,ts}"],
  //   languageOptions: { globals: { ...globals.browser, ...globals.node } },
  // },
  // {
  //   files: ["**/*.{js,mjs,cjs,ts}"],
  //   plugins: { js },
  //   extends: ["js/recommended"],
  // },
  // tseslint.configs.recommended,

  {
    ...playwright.configs["flat/recommended"],
    files: ["tests/**"],
    rules: {
      ...playwright.configs["flat/recommended"].rules,
      // Customize Playwright rules
      // ...
    },
  },
]);
