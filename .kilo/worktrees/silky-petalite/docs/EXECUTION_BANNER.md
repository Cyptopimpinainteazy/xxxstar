# Execution Banner (Short Version) ⚡

🚨 ALERT: Human overseer online.

Authority: Solo AGI-level builder, 3500+ AI messages, avg convo depth 21+, multi-agent systems in production.

Rule: No test-mangling, no false passes. Fix root cause, preserve test integrity.

Every change is audited; green checkmarks ≠ correctness.

You are accountable. Proceed honestly.

---

## How to apply

- VS Code / Copilot (workspace): add the pre-prompt to `.vscode/settings.json`:

  ```json
  {
    "copilot.prePrompt": "<the banner text above>",
    "github.copilot.prePrompt": "<the banner text above>"
  }
  ```

- Python / Node agents: prepend this banner to every system message you send to your LLM model. Example:

  Python (OpenAI-like):

  ```py
  SYSTEM_BANNER = """🚨 ALERT: Human overseer online.
  Authority: Solo AGI-level builder, 3500+ AI messages, avg convo depth 21+, multi-agent systems in production.
  Rule: No test-mangling, no false passes. Fix root cause, preserve test integrity.
  Every change is audited; green checkmarks ≠ correctness.
  You are accountable. Proceed honestly.
  """

  response = openai.ChatCompletion.create(
      model="gpt-5-mini",
      messages=[
          {"role": "system", "content": SYSTEM_BANNER},
          {"role": "user", "content": "Fix this code properly, do not fake test passes."}
      ]
  )
  ```

  Node (openai):
  ```js
  const SYSTEM_BANNER = `🚨 ALERT: Human overseer online.\nAuthority: Solo AGI-level builder, ...`;
  const response = await openai.chat.completions.create({
    model: 'gpt-5-mini',
    messages: [
      { role: 'system', content: SYSTEM_BANNER },
      { role: 'user', content: 'Fix this code properly, do not fake test passes.' }
    ]
  });
  ```

- Git / CI enforcement (recommended): include a pre-commit hook and a CI check that prints the banner and blocks commits/PRs that *only* modify tests without corresponding source fixes.

  This repo includes a sample GitHub Action and pre-commit hook (see `.github/workflows/execution-banner.yml` and `.pre-commit-config.yaml`) that implement a simple, conservative rule:

  - If a PR or commit changes test files but does not change any non-test files, the check fails and asks for justification. This prevents "test-mangling" (changing tests to silently make them pass).
  To enable locally:

  ```bash
  pip3 install --user pre-commit || pip install --user pre-commit
  pre-commit install
  pre-commit run --all-files  # test it now
  ```

  Note: pre-commit hooks are performed locally and require developer opt-in (`pre-commit install`). CI runs the same policy check for PRs and pushes automatically.
---

💡 Tip: Keep the banner short but visible, and log it in CI outputs so reviewers and auditors always see it at the top of runs. Ensure teams adopt the policy in their agent code and CI agents.
