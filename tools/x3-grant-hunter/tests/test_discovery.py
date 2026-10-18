import json
import pathlib
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import discovery
import grantops

LEAD = {'id': 'test-1', 'name': 'Research award', 'official_url': 'https://example.org/award', 'category': 'research'}
SOURCE = 'https://example.org/feed.json'
CONFIG = {'sources': [{'url': SOURCE, 'allowed_hosts': ['example.org']}]}

class DiscoveryTests(unittest.TestCase):
    def test_dry_run(self):
        with tempfile.TemporaryDirectory() as directory:
            db = pathlib.Path(directory) / 'test.db'
            result = discovery.run(CONFIG, db, fetcher=lambda u, h: [LEAD])
            self.assertEqual(result['count'], 1)
            self.assertFalse(db.exists())
            self.assertFalse(result['eligibility_verified'])

    def test_review_import(self):
        with tempfile.TemporaryDirectory() as directory:
            db = pathlib.Path(directory) / 'test.db'
            discovery.run(CONFIG, db, dry_run=False, fetcher=lambda u, h: [LEAD])
            self.assertEqual(len(grantops.pipeline(path=db)), 1)
            self.assertEqual(grantops.pipeline(path=db)[0]['stage'], 'discovered')

    def test_dedup(self):
        self.assertEqual(discovery.run(CONFIG, fetcher=lambda u, h: [LEAD, LEAD])['count'], 1)

    def test_reject_invalid(self):
        bad = dict(LEAD, category='arbitrary')
        result = discovery.run(CONFIG, fetcher=lambda u, h: [bad])
        self.assertEqual(result['count'], 0)
        self.assertEqual(result['sources'][0]['status'], 'error')

    def test_reject_local_and_http(self):
        with self.assertRaises(ValueError):
            discovery.validate_source('http://example.org/feed', {'example.org'})
        with self.assertRaises(ValueError):
            discovery.validate_source('https://localhost/feed', {'localhost'})
        with self.assertRaises(ValueError):
            discovery.validate_source('https://example.org/feed', {'other.org'})

    def test_reject_private_resolution(self):
        with patch('discovery.socket.getaddrinfo', return_value=[(2,1,6,'',('127.0.0.1',443))]):
            with self.assertRaises(ValueError):
                discovery.validate_source(SOURCE, {'example.org'})

    def test_limit(self):
        with self.assertRaises(ValueError):
            discovery.normalize([LEAD] * 501, SOURCE)

if __name__ == '__main__':
    unittest.main()
