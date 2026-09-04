/** @type {import('ts-jest').JestConfigWithTsJest} */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  roots: ['<rootDir>/src'],
  testMatch: ['**/__tests__/**/*.test.ts', '**/__tests__/**/*.test.tsx'],
  testPathIgnorePatterns: ['/node_modules/', '/dist/', '/build/', '/.next/'],
  // ts-jest overrides the Next.js tsconfig (module ESNext / moduleResolution
  // bundler) so emitted output is CommonJS that Node can require.
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
  testTimeout: 10000,
};
