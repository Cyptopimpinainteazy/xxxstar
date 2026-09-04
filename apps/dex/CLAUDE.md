### 🔄 Project Awareness & Context
- **Always read `PLANNING.md`** at the start of a new conversation to understand the project's architecture, goals, style, and constraints.
- **Check `TASK.md`** before starting a new task. If the task isn't listed, add it with a brief description and today's date.
- **Use consistent naming conventions, file structure, and architecture patterns** as described in `PLANNING.md`.
- This is a **Next.js TypeScript DEX (Decentralized Exchange)** frontend for X3 Chain.

### 🧱 Code Structure & Modularity
- **Never create a file longer than 500 lines of code.** If a file approaches this limit, refactor by splitting it into modules or helper files.
- **Organize code into clearly separated modules**, grouped by feature or responsibility.
  For Next.js DEX app this looks like:
    - `app/` - Next.js App Router pages and layouts
    - `components/` - Reusable React components
    - `hooks/` - Custom React hooks
    - `lib/` - Utility functions and API clients
    - `types/` - TypeScript type definitions
    - `styles/` - CSS and styling files
- **Use clear, consistent imports** (prefer relative imports within packages).
- **Use TypeScript** for type safety throughout.

### 🧪 Testing & Reliability
- **Always create unit tests for new features** (components, functions, utilities).
- **After updating any logic**, check whether existing unit tests need to be updated. If so, do it.
- **Tests should live in a `/tests` folder** or co-located with components as `*.test.tsx`.
  - Include at least:
    - 1 test for expected use
    - 1 edge case
    - 1 failure case
- Use **Playwright** for E2E tests and **Jest/Vitest** for unit tests.

### ✅ Task Completion
- **Mark completed tasks in `TASK.md`** immediately after finishing them.
- Add new sub-tasks or TODOs discovered during development to `TASK.md` under a "Discovered During Work" section.

### 📎 Style & Conventions
- **Use TypeScript** as the primary language.
- **Follow ESLint rules** defined in `.eslintrc.json` or `eslint.config.mjs`.
- **Use Tailwind CSS** for styling (configured in `postcss.config.js`).
- **Use Next.js App Router** patterns (Server Components by default, Client Components when needed).
- Write **JSDoc comments for every function** using the standard format:
  ```typescript
  /**
   * Brief summary.
   * @param param1 - Description.
   * @returns Description.
   */
  ```
- **Components** should use `.tsx` extension
- **Hooks** should use `.ts` or `.tsx` extension and start with `use`

### 📚 Documentation & Explainability
- **Update `README.md`** when new features are added, dependencies change, or setup steps are modified.
- **Comment non-obvious code** and ensure everything is understandable to a mid-level developer.
- When writing complex logic, **add an inline `// Reason:` comment** explaining the why, not just the what.

### 🧠 AI Behavior Rules
- **Never assume missing context. Ask questions if uncertain.**
- **Never hallucinate libraries or functions** – only use known, verified JavaScript/TypeScript packages.
- **Always confirm file paths and module names** exist before referencing them in code or tests.
- **Never delete or overwrite existing code** unless explicitly instructed to or if part of a task from `TASK.md`.
- This project uses **Next.js** with **App Router**, **TypeScript**, and **Tailwind CSS**.

### 🔗 Integration Points
- **Blockchain**: X3 Chain integration via Web3 libraries
- **Backend**: API calls to X3 Chain RPC endpoints
- **Wallet**: Web3 wallet connection (MetaMask, WalletConnect, etc.)
- **Build**: Next.js build system with TypeScript compiler

### 📁 Key Files Reference
- `app/page.tsx` - Main page component
- `app/layout.tsx` - Root layout with providers
- `package.json` - Dependencies and scripts
- `next.config.js` - Next.js configuration
- `tsconfig.json` - TypeScript configuration
- `.eslintrc.json` / `eslint.config.mjs` - Linting rules
- `postcss.config.js` - PostCSS/Tailwind configuration