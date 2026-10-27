"""Read-only queue audit: wrong PR identity must never imply cancellation."""
import unittest

from queue_audit import classify, audit


HEAD = 'a' * 40
OLD = 'b' * 40
OPEN = [{'number': 700, 'head': {'sha': HEAD}}]


def run(run_id, sha=HEAD, status='queued', links=None, workflow='.github/workflows/local-ci.yml'):
    return {'id': run_id, 'head_sha': sha, 'status': status,
            'path': workflow, 'pull_requests': [{'number': 700}] if links is None else links}


class QueueAuditTests(unittest.TestCase):
    def test_current_queued_head_is_preserved(self):
        report = audit(OPEN, [run(100)])
        self.assertEqual(report['runs'][0]['classification'], 'KEEP_CURRENT_HEAD')
        self.assertFalse(report['cancel_authorized'])
        self.assertEqual(report['runs'][0]['action'], 'NONE')

    def test_outdated_queued_run_is_review_only(self):
        report = audit(OPEN, [run(100, OLD)])
        self.assertEqual(report['runs'][0]['classification'], 'REVIEW_OUTDATED_HEAD')
        self.assertFalse(report['runs'][0]['cancel_authorized'])

    def test_in_progress_old_head_is_not_suggested_for_cancellation(self):
        report = audit(OPEN, [run(100, OLD, 'in_progress')])
        self.assertEqual(report['runs'][0]['classification'], 'KEEP_IN_PROGRESS')

    def test_unknown_pr_link_is_not_labelled_stale(self):
        report = audit(OPEN, [run(100, OLD, links=[])])
        self.assertEqual(report['runs'][0]['classification'], 'UNKNOWN_IDENTITY')

    def test_duplicate_queued_run_is_review_only(self):
        report = audit(OPEN, [run(100), run(101)])
        result = {item['id']: item['classification'] for item in report['runs']}
        self.assertEqual(result[101], 'KEEP_CURRENT_HEAD')
        self.assertEqual(result[100], 'REVIEW_DUPLICATE')

    def test_two_distinct_workflows_are_not_duplicates(self):
        report = audit(OPEN, [run(100), run(101, workflow='.github/workflows/other.yml')])
        self.assertTrue(all(item['classification'] == 'KEEP_CURRENT_HEAD' for item in report['runs']))

    def test_partial_or_invalid_data_cannot_create_authority(self):
        for prs, runs in [(None, []), ([], None), (OPEN, [{'status': 'queued'}])]:
            with self.subTest(prs=prs, runs=runs):
                with self.assertRaises(ValueError):
                    audit(prs, runs)

    def test_unsupported_status_is_rejected(self):
        with self.assertRaises(ValueError):
            audit(OPEN, [run(1, status='success')])

    def test_report_never_claims_runner_health_or_release_readiness(self):
        report = audit(OPEN, [run(100)], source='fixture')
        self.assertFalse(report['live_registry_verified'])
        self.assertFalse(report['release_authorized'])
        self.assertFalse(report['cancel_authorized'])

    def test_duplicate_open_pr_identity_refused(self):
        with self.assertRaises(ValueError):
            audit(OPEN + OPEN, [run(100)])


if __name__ == '__main__':
    unittest.main()
