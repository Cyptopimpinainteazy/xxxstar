"""Real process crash recovery for the router's SQLite store."""
import importlib.util
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import unittest

ROUTER_PATH = Path(__file__).with_name("router.py")
spec = importlib.util.spec_from_file_location("durability_router", ROUTER_PATH)
router_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(router_module)


class RouterDurabilityTests(unittest.TestCase):
    def test_sigkill_preserves_committed_usage_and_rolls_back_pending_write(self):
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / "state" / "usage.sqlite3"
            program = '''
import importlib.util, sys
spec = importlib.util.spec_from_file_location("router", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
router = module.Router({"provider_cooldown_seconds": 3600}, sys.argv[2])
router.db.execute("INSERT INTO usage (day, agent, cost_usd) VALUES (?, ?, ?)",
                  ("2026-10-01", "committed", 0.25))
router.db.commit()
router.note_provider_failure("primary", "provider unavailable")
router.db.execute("INSERT INTO usage (day, agent, cost_usd) VALUES (?, ?, ?)",
                  ("2026-10-01", "pending", 0.5))
print("ready", flush=True)
sys.stdin.readline()
'''
            child = subprocess.Popen([sys.executable, "-c", program, str(ROUTER_PATH), str(database)],
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.PIPE, text=True)
            try:
                readable, _, _ = select.select([child.stdout], [], [], 5)
                self.assertTrue(readable, "child did not reach its pending transaction")
                self.assertEqual(child.stdout.readline().strip(), "ready")
                child.kill()
                child.wait(timeout=5)
                self.assertLess(child.returncode, 0, "child must have died from a signal")
                restarted = router_module.Router({}, database)
                try:
                    self.assertEqual(restarted.db.execute("PRAGMA integrity_check").fetchone(), ("ok",))
                    self.assertEqual(restarted.db.execute("SELECT agent, cost_usd FROM usage").fetchall(),
                                     [("committed", 0.25)])
                    self.assertGreater(restarted.provider_cooldown("primary"), 0)
                    restarted.db.execute("INSERT INTO usage (agent) VALUES (?)", ("after-restart",))
                    restarted.db.commit()
                    self.assertEqual(restarted.db.execute("SELECT COUNT(*) FROM usage").fetchone()[0], 2)
                finally:
                    restarted.db.close()
            finally:
                if child.poll() is None:
                    child.kill()
                child.communicate(timeout=5)
