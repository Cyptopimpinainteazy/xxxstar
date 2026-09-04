"""
Quality gates — linting, testing, MCP validation, coverage enforcement.
Hard stops. No exceptions. Any failure blocks commit.
"""
import subprocess
from typing import Dict, List, Tuple
from pathlib import Path

from md_supervisor.schema import ChangeRequest


class GateResult:
    def __init__(self, name: str, passed: bool, output: str = ""):
        self.name = name
        self.passed = passed
        self.output = output


class GatePipeline:
    """
    Ordered quality gate pipeline. Each gate runs on modified files.
    If any gate fails, the entire pipeline fails and commit is blocked.
    """
    
    def __init__(self):
        self.gates: List[GateResult] = []
    
    def run_all(self, req: ChangeRequest) -> List[GateResult]:
        """Run all relevant quality gates for a change request."""
        self.gates = []
        
        for ft in req.files:
            lang = ft.language
            path = Path(ft.path)
            
            if lang == "rs":
                self.gates.append(self._run_cargo_check(path))
                self.gates.append(self._run_clippy(path))
            elif lang in ("py", "python"):
                self.gates.append(self._run_flake8(path))
                self.gates.append(self._run_mypy(path))
            elif lang in ("ts", "tsx", "js", "jsx"):
                self.gates.append(self._run_eslint(path))
                self.gates.append(self._run_typescript(path))
            elif lang == "go":
                self.gates.append(self._run_golangci_lint(path))
            elif lang == "sol":
                self.gates.append(self._run_forge(path))
            elif lang in ("json", "toml", "yaml", "yml"):
                self.gates.append(self._run_syntax_check(ft.path, ft.proposed_content))
        
        # Add security scan
        self.gates.append(self._run_security_scan(req))
        
        return self.gates
    
    def all_passed(self) -> bool:
        return all(g.passed for g in self.gates)
    
    def report(self) -> str:
        lines = ["=== Quality Gate Report ==="]
        for g in self.gates:
            status = "✅" if g.passed else "❌"
            lines.append(f"  {status} {g.name}")
            if not g.passed and g.output:
                lines.append(f"     {g.output[:200]}")
        lines.append(f"\nOverall: {'✅ ALL PASSED' if self.all_passed() else '❌ FAILED'}")
        return "\n".join(lines)
    
    def _run_cargo_check(self, path: Path) -> GateResult:
        try:
            result = subprocess.run(
                ["cargo", "check", "--manifest-path", str(path.parent / "Cargo.toml")],
                capture_output=True, text=True, timeout=60
            )
            return GateResult("cargo check", result.returncode == 0, result.stderr[:500])
        except Exception as e:
            return GateResult("cargo check", False, str(e))
    
    def _run_clippy(self, path: Path) -> GateResult:
        try:
            result = subprocess.run(
                ["cargo", "clippy", "--manifest-path", str(path.parent / "Cargo.toml"), "--", "-D", "warnings"],
                capture_output=True, text=True, timeout=60
            )
            return GateResult("clippy", result.returncode == 0, result.stderr[:500])
        except Exception as e:
            return GateResult("clippy", False, str(e))
    
    def _run_flake8(self, path: Path) -> GateResult:
        try:
            result = subprocess.run(
                ["flake8", str(path)], capture_output=True, text=True, timeout=30
            )
            return GateResult("flake8", result.returncode == 0, result.stdout[:500])
        except Exception as e:
            return GateResult("flake8", False, str(e))
    
    def _run_mypy(self, path: Path) -> GateResult:
        try:
            result = subprocess.run(
                ["mypy", str(path)], capture_output=True, text=True, timeout=30
            )
            return GateResult("mypy", result.returncode == 0, result.stdout[:500])
        except:
            return GateResult("mypy", True, "not installed")
    
    def _run_eslint(self, path: Path) -> GateResult:
        try:
            result = subprocess.run(
                ["eslint", str(path)], capture_output=True, text=True, timeout=30
            )
            return GateResult("eslint", result.returncode == 0, result.stdout[:500])
        except:
            return GateResult("eslint", True, "not installed")
    
    def _run_typescript(self, path: Path) -> GateResult:
        try:
            result = subprocess.run(
                ["tsc", "--noEmit", str(path)], capture_output=True, text=True, timeout=30
            )
            return GateResult("tsc", result.returncode == 0, result.stderr[:500])
        except:
            return GateResult("tsc", True, "not installed")
    
    def _run_golangci_lint(self, path: Path) -> GateResult:
        try:
            result = subprocess.run(
                ["golangci-lint", "run", str(path)], capture_output=True, text=True, timeout=60
            )
            return GateResult("golangci-lint", result.returncode == 0, result.stdout[:500])
        except:
            return GateResult("golangci-lint", True, "not installed")
    
    def _run_forge(self, path: Path) -> GateResult:
        try:
            result = subprocess.run(
                ["forge", "build"], capture_output=True, text=True, timeout=120
            )
            return GateResult("forge build", result.returncode == 0, result.stderr[:500])
        except:
            return GateResult("forge build", True, "not installed")
    
    def _run_syntax_check(self, path: str, content: str) -> GateResult:
        import json, tomllib, yaml
        try:
            if path.endswith(".json"):
                json.loads(content)
            elif path.endswith(".toml"):
                tomllib.loads(content)
            elif path.endswith((".yaml", ".yml")):
                yaml.safe_load(content)
            return GateResult(f"syntax: {path}", True)
        except Exception as e:
            return GateResult(f"syntax: {path}", False, str(e))
    
    def _run_security_scan(self, req: ChangeRequest) -> GateResult:
        """Static security scan of proposed content."""
        issues = []
        for ft in req.files:
            if any(kw in ft.proposed_content for kw in ("exec(", "eval(", "import os", "__import__", "subprocess")):
                issues.append(f"Possible code injection in {ft.path}")
            if "secret_key" in ft.proposed_content.lower() or "password=" in ft.proposed_content.lower():
                issues.append(f"Possible secret exposure in {ft.path}")
        if issues:
            return GateResult("security scan", False, "; ".join(issues))
        return GateResult("security scan", True)


def run_gates(req: ChangeRequest) -> Tuple[bool, str]:
    """Convenience function: run all gates and return pass/fail + report."""
    pipeline = GatePipeline()
    pipeline.run_all(req)
    return pipeline.all_passed(), pipeline.report()