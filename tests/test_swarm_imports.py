"""Import-order regressions for the production swarm package graph."""
import subprocess
import sys
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class SwarmImportTests(unittest.TestCase):
    def run_imports(self, source):
        result = subprocess.run([sys.executable, '-c', source], cwd=ROOT,
                                capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_schema_import_does_not_construct_core_graph(self):
        self.run_imports('''
import sys
from swarm.core.enums import Domain
assert "swarm.core.agent" not in sys.modules
assert "swarm.core.lifecycle" not in sys.modules
''')

    def test_reaper_then_agent(self):
        self.run_imports('''
from swarm.reaper import ReaperEngine, ScarPropagator
from swarm.core import Agent
from swarm.core.agent import Agent as Implementation
assert Agent is Implementation
''')

    def test_self_improvement_then_agent(self):
        self.run_imports('''
from swarm.self_improve.schema import Scar
from swarm.core import Agent, EpochOrchestrator
from swarm.core.lifecycle import EpochOrchestrator as Implementation
assert EpochOrchestrator is Implementation
''')

    def test_agent_then_reaper(self):
        self.run_imports('''
from swarm.core import Agent, AgentConfig, ActionResult, Consequence, EpochStats
from swarm.reaper import ReaperEngine, ScarPropagator
from swarm.core.agent import Agent as Implementation
assert Agent is Implementation
''')

    def test_public_exports_and_unknown_attribute(self):
        self.run_imports('''
import swarm.core as core
namespace = {}
exec("from swarm.core import *", namespace)
for name in core.__all__:
    assert namespace[name] is getattr(core, name)
try:
    core.nonexistent_export
except AttributeError:
    pass
else:
    raise AssertionError("unknown package attribute must fail")
''')


if __name__ == '__main__':
    unittest.main()
