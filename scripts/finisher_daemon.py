#!/usr/bin/env python3
"""
☢️ YOLO FINISHER v5.0 — DROP-FOLDER DAEMON
==========================================
Self-governing nuclear finalization engine.

Drop a repository into the watched folder.
The daemon runs all agents in sequence.
Loops until the repo scores 100/100.

Usage:
    python scripts/finisher_daemon.py [--watch-dir ./drop] [--work-dir ./workspace]
"""

import argparse
import json
import pathlib
import shutil
import time
from datetime import datetime

# ──────────────────────────────────────────
# AGENT STACK (Nuclear Execution Order)
# ──────────────────────────────────────────
AGENTS = [
    "CARTOGRAPHER",
    "ARCHAEOLOGIST",
    "BREAKER",
    "AUDITOR",
    "INTENT_ANALYST",
    "INTEGRATOR",
    "VERIFIER",
    "FIXER",
    "ECONOMIST",
    "CHAOS_ENGINE",
    "COMPLETION_JUDGE",
]

# Map agent names to their prompt files
AGENT_PROMPTS = {
    "CARTOGRAPHER":     "finisher-cartographer.prompt.md",
    "ARCHAEOLOGIST":    "finisher-archaeologist.prompt.md",
    "BREAKER":          "finisher-breaker.prompt.md",
    "AUDITOR":          "finisher-auditor.prompt.md",
    "INTENT_ANALYST":   "finisher-intent-analyst.prompt.md",
    "INTEGRATOR":       "finisher-integrator.prompt.md",
    "VERIFIER":         "finisher-verifier.prompt.md",
    "FIXER":            "finisher-fixer.prompt.md",
    "ECONOMIST":        "finisher-economist.prompt.md",
    "CHAOS_ENGINE":     "finisher-chaos-engine.prompt.md",
    "COMPLETION_JUDGE": "finisher-completion-judge.prompt.md",
}

MAX_LOOPS = 10  # Safety valve — prevent infinite loops


def log(msg: str, level: str = "INFO") -> None:
    """Structured logging."""
    timestamp = datetime.utcnow().isoformat() + "Z"
    print(f"[{timestamp}] [{level}] {msg}", flush=True)


def run_agent(agent: str, repo_path: pathlib.Path, prompts_dir: pathlib.Path) -> bool:
    """
    Run a single agent against the repo.
    Returns True if the agent succeeded, False if it failed.

    NOTE: This is a skeleton — bind to your actual agent runner:
    - OpenRouter API call
    - Local Ollama call
    - Claude/Gemini API call
    - Or any multi-agent framework
    """
    prompt_file = prompts_dir / AGENT_PROMPTS.get(agent, "")

    log(f"☢️  Running {agent}", "AGENT")

    if not prompt_file.exists():
        log(f"⚠️  Prompt file not found: {prompt_file}", "WARN")
        return False

    # ──────────────────────────────────────
    # BIND YOUR AGENT RUNNER HERE
    # ──────────────────────────────────────
    # Example with a hypothetical run_agent.py:
    #
    # result = subprocess.run(
    #     ["python", "run_agent.py", agent, str(repo_path), str(prompt_file)],
    #     capture_output=True,
    #     text=True,
    #     cwd=str(repo_path),
    # )
    #
    # if result.returncode != 0:
    #     log(f"❌ {agent} failed:\n{result.stdout}\n{result.stderr}", "ERROR")
    #     return False
    #
    # log(f"✅ {agent} completed", "AGENT")
    # return True
    # ──────────────────────────────────────

    # Placeholder — reads prompt and confirms it exists
    log(f"  📄 Prompt: {prompt_file.name} ({prompt_file.stat().st_size} bytes)", "AGENT")
    log(f"  📁 Repo:   {repo_path}", "AGENT")
    log(f"  ⏳ Agent {agent} ready — bind to your model runner", "AGENT")

    return True


def check_completion_score(repo_path: pathlib.Path) -> int:
    """
    Check the completion score from SCORE_REPORT.json.
    Returns the score (0-100).
    """
    score_file = repo_path / "SCORE_REPORT.json"
    if score_file.exists():
        try:
            data = json.loads(score_file.read_text())
            return data.get("total_score", 0)
        except (json.JSONDecodeError, KeyError):
            return 0
    return 0


def process_repo(repo: pathlib.Path, work_dir: pathlib.Path, prompts_dir: pathlib.Path) -> None:
    """Process a single repository through the full agent stack."""
    repo_name = repo.stem
    repo_path = work_dir / repo_name
    log(f"🔥 Processing: {repo_name}", "DAEMON")

    # Copy to workspace
    if repo_path.exists():
        shutil.rmtree(repo_path)
    shutil.copytree(repo, repo_path)

    loop_count = 0
    while loop_count < MAX_LOOPS:
        loop_count += 1
        log(f"🔄 Loop {loop_count}/{MAX_LOOPS}", "DAEMON")

        all_passed = True
        for agent in AGENTS:
            try:
                success = run_agent(agent, repo_path, prompts_dir)
                if not success:
                    all_passed = False
                    log(f"💥 {agent} failed — restarting loop", "DAEMON")
                    break
            except Exception as e:
                all_passed = False
                log(f"💥 {agent} crashed: {e}", "ERROR")
                break

        if not all_passed:
            continue

        # Check completion score
        score = check_completion_score(repo_path)
        if score >= 100:
            log(f"✅ REPO FINALIZED — Score: {score}/100", "DAEMON")
            return
        else:
            log(f"📊 Score: {score}/100 — looping again", "DAEMON")

    log(f"⚠️  Max loops ({MAX_LOOPS}) reached for {repo_name}", "WARN")


def main() -> None:
    parser = argparse.ArgumentParser(description="☢️ YOLO FINISHER v5.0 — Drop-Folder Daemon")
    parser.add_argument("--watch-dir", default="./drop", help="Directory to watch for repos")
    parser.add_argument("--work-dir", default="./workspace", help="Working directory for processing")
    parser.add_argument("--prompts-dir", default="./.github/prompts", help="Directory containing agent prompts")
    parser.add_argument("--poll-interval", type=int, default=5, help="Seconds between polls")
    args = parser.parse_args()

    watch_dir = pathlib.Path(args.watch_dir)
    work_dir = pathlib.Path(args.work_dir)
    prompts_dir = pathlib.Path(args.prompts_dir)

    watch_dir.mkdir(exist_ok=True)
    work_dir.mkdir(exist_ok=True)

    log("☢️  YOLO FINISHER v5.0 — NUCLEAR MODE ACTIVE", "DAEMON")
    log(f"   Watch: {watch_dir.resolve()}", "DAEMON")
    log(f"   Work:  {work_dir.resolve()}", "DAEMON")
    log(f"   Prompts: {prompts_dir.resolve()}", "DAEMON")

    while True:
        for repo in watch_dir.iterdir():
            if repo.is_dir() and not repo.name.startswith("."):
                try:
                    process_repo(repo, work_dir, prompts_dir)
                except Exception as e:
                    log(f"💥 Failed to process {repo.name}: {e}", "ERROR")
                finally:
                    # Clean up watched copy after processing
                    if repo.exists():
                        shutil.rmtree(repo)
        time.sleep(args.poll_interval)


if __name__ == "__main__":
    main()
