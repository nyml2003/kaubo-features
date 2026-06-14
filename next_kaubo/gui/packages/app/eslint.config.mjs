import js from "@eslint/js";
import ts from "typescript-eslint";

export default [
  js.configs.recommended,
  ...ts.configs.strictTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    files: ["**/*.{ts,tsx}"],
    rules: {
      // Solid requires JSX. Keep it clean.
      "@typescript-eslint/no-non-null-assertion": "error",
      "@typescript-eslint/no-empty-object-type": "off",
    },
  },
];
