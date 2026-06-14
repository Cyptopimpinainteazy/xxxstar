import tseslint from 'typescript-eslint';
import react from 'eslint-plugin-react';

export default [
  {
    ignores: ['.next/**', 'node_modules/**', 'out/**', 'dist/**'],
  },
  ...tseslint.configs.recommended,
  {
    plugins: {
      react,
    },
    languageOptions: {
      parserOptions: {
        ecmaVersion: 'latest',
      },
    },
  },
];