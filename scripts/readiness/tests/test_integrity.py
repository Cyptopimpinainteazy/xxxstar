"""Regression contracts for evidence freshness, dependency closure and atomic publication."""
import test_workflow as base
workflow = base.workflow
import json
from pathlib import Path
import subprocess
import sys
import unittest

class IntegrityTests(unittest.TestCase):
    setUp = base.WorkflowTests.setUp
    run_check = base.WorkflowTests.run_check
    review = base.WorkflowTests.review
    result = base.WorkflowTests.result
    def test_adding_required_check_invalidates_existing_task_closure(self):
        self.run_check(); self.review()
        workflow.add_check(self.state, 'second', [sys.executable, '-c', 'print(2)'], ['FIX-C01'])
        self.assertNotEqual(self.result()['tasks'][0]['status'], 'completed')

    def test_dependency_must_close_before_dependent_task(self):
        self.state['tasks'].append({'id': 'FIX-H01', 'description': 'Dependent work', 'severity': 'High',
                                    'acceptance': 'Dependent acceptance', 'dependencies': ['FIX-C01'], 'requested_status': 'planned'})
        workflow.add_check(self.state, 'second', [sys.executable, '-c', 'print(2)'], ['FIX-H01'])
        workflow.run_check(self.state, self.repo, self.store, 'second')
        workflow.review(self.state, self.repo, self.store, 'FIX-H01', 'closure', 'Reviewer', 'Dependency reviewed', ['second'])
        self.assertEqual(self.result()['tasks'][1]['status'], 'awaiting_verification')
        self.run_check(); self.review()
        self.assertEqual(self.result()['tasks'][1]['status'], 'completed')

    def test_unknown_dependency_fails_closed(self):
        self.state['tasks'][0]['dependencies'] = ['FIX-NOT-REAL']
        with self.assertRaises(ValueError):
            self.result()

    def test_source_modified_by_check_cannot_earn_credit(self):
        workflow.add_check(self.state, 'reject', [sys.executable, '-c', 'from pathlib import Path; Path("logic.py").write_text("changed")'], ['FIX-C01'])
        self.run_check()
        self.assertEqual(self.result()['checks'][0]['status'], 'invalid')

    def test_deleted_receipt_is_invalid_not_passed(self):
        receipt = self.run_check()
        (self.store / 'evidence' / (receipt['id'] + '.json')).unlink()
        self.assertEqual(self.result()['checks'][0]['status'], 'invalid')

    def test_changed_acceptance_invalidates_review(self):
        self.run_check(); self.review()
        self.state['tasks'][0]['acceptance'] = 'A stronger acceptance condition'
        self.assertNotEqual(self.result()['tasks'][0]['status'], 'completed')

    def test_missing_executable_is_recorded_and_does_not_pass(self):
        workflow.add_check(self.state, 'reject', ['/nonexistent/x3-command'], ['FIX-C01'])
        receipt = self.run_check()
        self.assertEqual(receipt['exit_code'], 127)
        self.assertEqual(self.result()['checks'][0]['status'], 'failed')

    def test_edited_snapshot_fails_checksum_validation(self):
        destination = workflow.refresh(self.state, self.repo, self.store, pdf=False)
        (destination / 'index.html').write_text('modified')
        with self.assertRaises(ValueError):
            workflow.verify_snapshot(destination)

    def test_failed_pdf_render_keeps_previous_snapshot_pointer(self):
        workflow.refresh(self.state, self.repo, self.store, pdf=False)
        before = (self.store / 'current.json').read_bytes()
        # A real over-height PDF table row provokes ReportLab's layout rejection.
        self.state['tasks'][0]['acceptance'] = 'long acceptance\n' * 1500
        with self.assertRaises(Exception):
            workflow.refresh(self.state, self.repo, self.store, pdf=True)
        self.assertEqual((self.store / 'current.json').read_bytes(), before)
        self.assertEqual(len(list((self.store / 'snapshots').glob('.building-*'))), 0)

    def test_watcher_does_not_publish_duplicate_unchanged_snapshots(self):
        workflow.atomic_json(self.store / 'state.json', self.state)
        process = subprocess.run([sys.executable, str(Path(workflow.__file__)), '--repo', str(self.repo),
                                  '--store', str(self.store), 'watch', '--no-pdf', '--interval', '1', '--iterations', '2'],
                                 capture_output=True, text=True, timeout=15)
        self.assertEqual(process.returncode, 0, process.stderr)
        self.assertEqual(len(list((self.store / 'snapshots').iterdir())), 1)

    def test_baseline_credit_expires_when_source_changes(self):
        snapshot = workflow.source_snapshot(self.repo)
        self.state['baseline'] = {'source_matched_at_import': True, 'source_fingerprint': snapshot['fingerprint'],
                                  'observed_at': workflow.now()}
        self.state['features'][0]['baseline_criteria'] = {'implemented': 1, 'wired': 1}
        self.assertEqual(self.result()['features'][0]['completion_percent'], 40)
        (self.repo / 'logic.py').write_text('VALUE=3')
        self.assertEqual(self.result()['features'][0]['completion_percent'], 0)

    def test_repeated_passing_run_requires_review_of_new_receipt(self):
        self.run_check(); self.review()
        self.run_check()
        self.assertNotEqual(self.result()['tasks'][0]['status'], 'completed')

    def test_invalid_severity_cannot_hide_a_security_finding(self):
        self.state['tasks'][0]['severity'] = 'none'
        with self.assertRaises(ValueError):
            self.result()

    def test_negative_weights_cannot_manipulate_readiness(self):
        self.state['subsystems'] = [{'subsystem': 'Safety', 'weight': -100}, {'subsystem': 'Other', 'weight': 200}]
        with self.assertRaises(ValueError):
            self.result()

    def test_duplicate_target_ids_are_rejected(self):
        self.state['tasks'].append(dict(self.state['tasks'][0]))
        with self.assertRaises(ValueError):
            self.result()
