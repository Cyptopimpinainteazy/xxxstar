### 🔄 Project Awareness & Context
- **Always read `PLANNING.md`** at the start of a new conversation to understand the project's architecture, goals, style, and constraints.
- **Check `TASK.md`** before starting a new task. If the task isn't listed, add it with a brief description and today's date.
- **Use consistent naming conventions, file structure, and architecture patterns** as described in `PLANNING.md`.
- This is a **frontend web project** for X3 Chain - a blockchain/crypto platform.

### 🧱 Code Structure & Modularity
- **Never create a file longer than 500 lines of code.** If a file approaches this limit, refactor by splitting it into modules or helper files.
- **Organize code into clearly separated modules**, grouped by feature or responsibility.
  For frontend components this looks like:
    - `src/components/` - Reusable UI components
    - `src/hooks/` - Custom React hooks
    - `src/services/` - API and external service integrations
    - `src/stores/` - State management
    - `css/` - Stylesheets
    - `js/` - Vanilla JavaScript utilities
- **Use clear, consistent imports** (prefer relative imports within packages).
- **Use ES6+ modules** for JavaScript code organization.

### 🧪 Testing & Reliability
- **Always create unit tests for new features** (components, functions, utilities).
- **After updating any logic**, check whether existing unit tests need to be updated. If so, do it.
- **Tests should live in a `/tests` folder** mirroring the main app structure.
  - Include at least:
    - 1 test for expected use
    - 1 edge case
    - 1 failure case
- Use **Playwright** for E2E tests and **Jest** for unit tests.

### ✅ Task Completion
- **Mark completed tasks in `TASK.md`** immediately after finishing them.
- Add new sub-tasks or TODOs discovered during development to `TASK.md` under a "Discovered During Work" section.

### 📎 Style & Conventions
- **Use JavaScript/JSX** as the primary languages.
- **Follow ESLint rules** for code quality.
- **Use Tailwind CSS** for styling (configured in `tailwind.config.js`).
- **Use React** for component-based UI architecture.
- Write **JSDoc comments for every function** using the standard format:
  ```javascript
  /**
   * Brief summary.
   * @param {string} param1 - Description.
   * @returns {type} Description.
   */
  ```
- **HTML files** should follow the `x3star-*.html` naming convention.

### 📚 Documentation & Explainability
- **Update `README.md`** when new features are added, dependencies changes, or setup steps are modified.
- **Comment non-obvious code** and ensure everything is understandable to a mid-level developer.
- When writing complex logic, **add an inline `// Reason:` comment** explaining the why, not just the what.

### 🧠 AI Behavior Rules
- **Never assume missing context. Ask questions if uncertain.**
- **Never hallucinate libraries or functions** – only use known, verified JavaScript/React packages.
- **Always confirm file paths and module names** exist before referencing them in code or tests.
- **Never delete or overwrite existing code** unless explicitly instructed to or if part of a task from `TASK.md`.
- This project uses **Vite** as the build tool and **PostCSS** for CSS processing.

### 🔗 Integration Points
- **Backend**: Node.js server (`server.js`, `server/`)
- **Data**: JSON data files (`data/`)
- **Blockchain**: X3 Chain integration (connects to parent x3-chain project)
- **Build**: Vite (`vite.config.js`), PostCSS (`postcss.config.js`), Tailwind CSS

### 📁 Key Files Reference
- `index.html` - Main entry point
- `package.json` - Dependencies and scripts
- `vite.config.js` - Build configuration
- `tailwind.config.js` - CSS framework config
- `src/` - React components and application logic
- `server/` - Backend Node.js services
- `data/` - JSON data stores