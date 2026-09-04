/** @type {import('ts-jest').JestConfigWithTsJest} */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'jsdom',
  roots: ['<rootDir>'],
  testMatch: ['**/__tests__/**/*.test.ts', '**/__tests__/**/*.test.tsx'],
  testPathIgnorePatterns: ['/node_modules/', '/dist/', '/build/'],
  // This package has no tsconfig.json; tell ts-jest how to handle TypeScript and
  // TSX (React .tsx tests) so the config is self-contained and independent of any
  // app-level tsconfig.
  transform: {
    '^.+\\.tsx?$': [
      'ts-jest',
      {
        tsconfig: {
          target: 'es2020',
          module: 'commonjs',
          moduleResolution: 'node',
          jsx: 'react-jsx',
          esModuleInterop: true,
          allowJs: true,
          strict: false,
          types: ['jest'],
        },
      },
    ],
  },
  setupFilesAfterEnv: ['@testing-library/jest-dom'],
  testTimeout: 10000,
};
