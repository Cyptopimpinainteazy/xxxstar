"""Behavioral contracts: real temporary Git trees and subprocesses; no production doubles."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / 'workflow.py'
sys.path.insert(0, str(MODULE.parent))
spec = importlib.util.spec_from_file_location('workflow', MODULE) if MODULE.exists() else None
workflow = importlib.util.module_from_spec(spec) if spec else None
if spec:
    spec.loader.exec_module(workflow)

class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(workflow, 'The approved readiness workflow is not implemented')
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        subprocess.run(['git', 'init', '-q', str(self.repo)], check=True)
        (self.repo / 'logic.py').write_text('VALUE = 1\n')
        self.store = self.repo / 'audit-artifacts/live'
        self.state = workflow.new_state(self.repo, self.store, tasks=[
            {'id': 'FIX-C01', 'description': 'Reject invalid input', 'severity': 'Critical',
             'acceptance': 'Invalid input is rejected without state changes', 'dependencies': []}],
            features=[{'id': 'FT01', 'feature': 'Input validation', 'subsystem': 'Safety'}],
            subsystems=[{'subsystem': 'Safety', 'weight': 100}])
        workflow.add_check(self.state, 'reject', [sys.executable, '-c', 'print("real check")'],
                           ['FIX-C01', 'FT01'], timeout=5)

    def run_check(self):
        return workflow.run_check(self.state, self.repo, self.store, 'reject')

    def review(self, target='FIX-C01', criterion='closure'):
        return workflow.review(self.state, self.repo, self.store, target, criterion,
                               'Protocol reviewer', 'Acceptance cases inspected', ['reject'])

    def result(self):
        return workflow.evaluate(self.state, self.repo, self.store)

    def test_checking_done_without_evidence_does_not_close_security_finding(self):
        self.state['tasks'][0]['requested_status'] = 'completed'
        result = self.result()
        self.assertEqual(result['tasks'][0]['status'], 'awaiting_verification')
        self.assertEqual(result['open_findings']['Critical'], 1)
        self.assertEqual(result['readiness_score'], 0)

    def test_real_execution_and_review_close_task_and_failed_rerun_reopens_it(self):
        self.run_check()
        self.review()
        self.assertEqual(self.result()['tasks'][0]['status'], 'completed')
        workflow.add_check(self.state, 'reject', [sys.executable, '-c', 'raise SystemExit(7)'],
                           ['FIX-C01', 'FT01'], timeout=5)
        receipt = self.run_check()
        self.assertEqual(receipt['exit_code'], 7)
        self.assertNotEqual(self.result()['tasks'][0]['status'], 'completed')

    def test_source_change_invalidates_passing_results_and_closure(self):
        self.run_check()
        self.review()
        (self.repo / 'logic.py').write_text('VALUE = 2\n')
        result = self.result()
        self.assertEqual(result['checks'][0]['status'], 'stale')
        self.assertNotEqual(result['tasks'][0]['status'], 'completed')

    def test_tampered_log_cannot_count_as_passing_evidence(self):
        receipt = self.run_check()
        self.review()
        (self.store / receipt['log']).write_text('changed evidence')
        self.assertEqual(self.result()['checks'][0]['status'], 'invalid')
        self.assertNotEqual(self.result()['tasks'][0]['status'], 'completed')

    def test_feature_score_requires_separate_review_for_each_criterion(self):
        self.run_check()
        self.review('FT01', 'tested')
        result = self.result()
        self.assertEqual(result['uncapped_score'], 20)
        self.assertEqual(result['features'][0]['completion_percent'], 20)
        for criterion in ['implemented', 'wired', 'executed', 'reproducible']:
            self.review('FT01', criterion)
        self.assertEqual(self.result()['readiness_score'], 20)  # open Critical cap
        self.review()
        self.assertEqual(self.result()['readiness_score'], 100)
        self.assertEqual(self.result()['release_decision'], 'NOT ASSESSED')

    def test_review_rejects_missing_or_failed_checks(self):
        with self.assertRaises(ValueError):
            self.review()

    def test_timeout_is_retained_as_failed_evidence(self):
        workflow.add_check(self.state, 'reject', [sys.executable, '-c', 'import time; time.sleep(5)'],
                           ['FIX-C01'], timeout=.05)
        receipt = self.run_check()
        self.assertEqual(receipt['exit_code'], 124)
        self.assertEqual(self.result()['checks'][0]['status'], 'failed')

    def test_check_cannot_claim_unrelated_feature(self):
        workflow.add_check(self.state, 'reject', [sys.executable, '-c', 'print(1)'], ['FIX-C01'], timeout=5)
        self.run_check()
        with self.assertRaises(ValueError):
            self.review('FT01', 'tested')

    def test_expired_evidence_does_not_keep_task_closed(self):
        self.run_check()
        self.review()
        self.state['max_evidence_age_days'] = 0
        self.assertNotEqual(self.result()['tasks'][0]['status'], 'completed')

    def test_published_snapshot_and_html_use_recalculated_values(self):
        self.run_check()
        self.review('FT01', 'tested')
        destination = workflow.refresh(self.state, self.repo, self.store, pdf=False)
        current = json.loads((self.store / 'current.json').read_text())
        report = json.loads((destination / 'summary.json').read_text())
        self.assertEqual(report['readiness_score'], 20)
        self.assertEqual(current['snapshot'], str(destination.relative_to(self.store)))
        self.assertIn('20/100', (destination / 'index.html').read_text())
        workflow.verify_snapshot(destination)

if __name__ == '__main__':
    unittest.main()
