name: "Base PRP Template v2 - Context-Rich with Validation Loops"
description: |

## Purpose
Template optimized for AI agents to implement features with sufficient context and self-validation capabilities to achieve working code through iterative refinement.

## Core Principles
1. **Context is King**: Include ALL necessary documentation, examples, and caveats
2. **Validation Loops**: Provide executable tests/lints the AI can run and fix
3. **Information Dense**: Use keywords and patterns from the codebase
4. **Progressive Success**: Start simple, validate, then enhance
5. **Global rules**: Be sure to follow all rules in CLAUDE.md

---

## Goal
[What needs to be built - be specific about the end state and desires]

## Why
- [Business value and user impact]
- [Integration with existing features]
- [Problems this solves and for whom]

## What
[User-visible behavior and technical requirements]

### Success Criteria
- [ ] [Specific measurable outcomes]

## All Needed Context

### Documentation & References (list all context needed to implement the feature)
```yaml
# MUST READ - Include these in your context window
- url: [Official API docs URL]
  why: [Specific sections/methods you'll need]
  
- file: [path/to/example.jsx]
  why: [Pattern to follow, gotchas to avoid]
  
- doc: [Library documentation URL] 
  section: [Specific section about common pitfalls]
  critical: [Key insight that prevents common errors]

- docfile: [PRPs/ai_docs/file.md]
  why: [docs that the user has pasted in to the project]

```

### Current Codebase tree (run `tree` in the root of the project) to get an overview of the codebase
```bash

```

### Desired Codebase tree with files to be added and responsibility of file
```bash

```

### Known Gotchas of our codebase & Library Quirks
```javascript
// CRITICAL: [Library name] requires [specific setup]
// Example: React components must use functional style with hooks
// Example: Tailwind classes must be defined in tailwind.config.js for custom values
// Example: Vite requires specific import syntax for assets
// Example: HTML files use vanilla JS, React files use JSX
```

## Implementation Blueprint

### Data models and structure

Create the core data models, we ensure type safety and consistency.
```javascript
Examples: 
 - PropTypes for React components
 - JSDoc type definitions
 - JSON schema for data files

```

### list of tasks to be completed to fulfill the PRP in the order they should be completed

```yaml
Task 1:
MODIFY src/components/existing.jsx:
  - FIND pattern: "const OldComponent"
  - INJECT after line containing "return"
  - PRESERVE existing prop signatures

CREATE src/components/new-feature.jsx:
  - MIRROR pattern from: src/components/similar.jsx
  - MODIFY component name and core logic
  - KEEP error handling pattern identical

...(...)

Task N:
...

```


### Per task pseudocode as needed added to each task
```javascript

// Task 1
// Pseudocode with CRITICAL details don't write entire code
function NewFeature({ prop1, prop2 }) {
  // PATTERN: Always validate props first (see src/components/validators.jsx)
  const validated = validateProps({ prop1, prop2 });  // throws on invalid
  
  // GOTCHA: This library requires connection pooling
  const { data, error } = useSWR(validated, fetcher);  // see src/hooks/useSWR.js
  
  // PATTERN: Loading state handling
  if (error) return <ErrorFallback error={error} />;
  if (!data) return <LoadingSpinner />;
  
  // PATTERN: Standardized response format
  return <FeatureDisplay data={data} />;  // see src/components/FeatureDisplay.jsx
}
```

### Integration Points
```yaml
COMPONENTS:
  - add to: src/components/
  - pattern: "export default function ComponentName(props) { ... }"
  
STYLES:
  - add to: css/x3-custom.css
  - pattern: ".component-name { ... }"
  
ROUTES:
  - add to: index.html or routing config
  - pattern: "<Route path='/feature' component={FeatureComponent} />"
  
DATA:
  - add to: data/
  - pattern: "{ key: 'value', ... }"
```

## Validation Loop

### Level 1: Syntax & Style
```bash
# Run these FIRST - fix any errors before proceeding
npm run lint  # ESLint check

# Expected: No errors. If errors, READ the error and fix.
```

### Level 2: Unit Tests each new feature/file/function use existing test patterns
```javascript
// CREATE test_new_feature.jsx with these test cases:
test('happy path', () => {
  // Basic functionality works
  const result = render(<NewFeature prop1="valid" />);
  expect(result.getByText('success')).toBeInTheDocument();
});

test('validation error', () => {
  // Invalid props handled gracefully
  expect(() => render(<NewFeature prop1="" />)).toThrow();
});

test('loading state', () => {
  // Handles loading gracefully
  const result = render(<NewFeature prop1="valid" />);
  expect(result.getByTestId('loading')).toBeInTheDocument();
});
```

```bash
# Run and iterate until passing:
npm test -- --testPathPattern=new_feature
# If failing: Read error, understand root cause, fix code, re-run (never mock to pass)
```

### Level 3: Integration Test
```bash
# Start the development server
npm run dev

# Test the feature in browser
curl http://localhost:5173/feature

# Expected: Page loads correctly
# If error: Check browser console for stack trace
```

## Final validation Checklist
- [ ] All tests pass: `npm test`
- [ ] No linting errors: `npm run lint`
- [ ] Build succeeds: `npm run build`
- [ ] Manual test successful: [specific browser/command]
- [ ] Error cases handled gracefully
- [ ] Logs are informative but not verbose
- [ ] Documentation updated if needed

---

## Anti-Patterns to Avoid
- ❌ Don't create new patterns when existing ones work
- ❌ Don't skip validation because "it should work"  
- ❌ Don't ignore failing tests - fix them
- ❌ Don't use class components when functional components with hooks exist
- ❌ Don't hardcode values that should be config
- ❌ Don't catch all exceptions - be specific
- ❌ Don't mix CSS-in-JS with Tailwind classes