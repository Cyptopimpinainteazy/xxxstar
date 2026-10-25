"""Isolated utility tests; these do not claim live switch interoperability."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('network_tools', Path(__file__).parents[1] / 'scripts/network_tools.py')
tools = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tools)


class UtilityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.state = self.root / 'state'
        self.patch = patch.object(tools, 'STATE', self.state)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.stdout = contextlib.redirect_stdout(io.StringIO())
        self.stdout.__enter__()
        self.addCleanup(self.stdout.__exit__, None, None, None)

    def test_backup_preserves_bytes_and_private_permissions(self):
        source = self.root / 'export'
        content = b'unit-test export\nbytes\x00\xff'
        source.write_bytes(content)
        self.assertEqual(tools.backup(source, 'x3-switch1'), 0)
        archived = next(self.state.glob('*/configuration.raw'))
        self.assertEqual(archived.read_bytes(), content)
        self.assertEqual(stat.S_IMODE(archived.stat().st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(archived.parent.stat().st_mode), 0o700)
        manifest = json.loads(archived.with_name('manifest.json').read_text())
        self.assertEqual(manifest['sha256'], tools.hashlib.sha256(content).hexdigest())
        self.assertEqual(manifest['completeness'], 'NOT VERIFIED')

    def test_backup_rejects_empty_source(self):
        source = self.root / 'empty'
        source.touch()
        with self.assertRaises(ValueError):
            tools.backup(source, 'x3-switch1')
        self.assertFalse(self.state.exists())

    def test_backup_rejects_symlink(self):
        source = self.root / 'export'
        source.write_text('test data')
        link = self.root / 'link'
        link.symlink_to(source)
        with self.assertRaises(ValueError):
            tools.backup(link, 'x3-switch1')

    def test_private_state_rejects_symlink(self):
        target = self.root / 'target'
        target.mkdir()
        self.state.symlink_to(target)
        with self.assertRaises(ValueError):
            tools.private_directory('test')

    def test_unreviewed_or_injected_ssh_target_never_executes(self):
        with patch.object(tools, 'run') as runner:
            for host, user, reviewed in [('192.168.0.2', 'admin', False),
                                         ('-oProxyCommand=evil', 'admin', True),
                                         ('host;reboot', 'admin', True),
                                         ('host', 'admin;reboot', True)]:
                with self.assertRaises(ValueError):
                    tools.collect(host, user, reviewed)
            runner.assert_not_called()

    def address_check(self, cidr=None, fail=False):
        def command(argv):
            output = '[]'
            if argv[:3] == ['ip', '-json', 'address'] and cidr:
                net = tools.ipaddress.ip_interface(cidr)
                output = json.dumps([{'addr_info': [{'family': 'inet', 'local': str(net.ip), 'prefixlen': net.network.prefixlen}]}])
            if argv[0] == 'docker':
                output = ''
            return {'argv': argv, 'exit_code': 1 if fail else 0, 'stdout': output, 'stderr': ''}
        with patch.object(tools, 'run', side_effect=command):
            return tools.validate()

    def test_overlap_is_rejected(self):
        self.assertEqual(self.address_check('10.30.20.7/24'), 1)

    def test_disjoint_local_address_passes_scoped_check(self):
        self.assertEqual(self.address_check('192.168.0.70/24'), 0)
        data = json.loads(next(self.state.glob('*/address-check.json')).read_text())
        self.assertTrue(data['deployment_approval'].startswith('BLOCKED'))

    def test_failed_discovery_never_reports_pass(self):
        self.assertEqual(self.address_check(fail=True), 2)
        data = json.loads(next(self.state.glob('*/address-check.json')).read_text())
        self.assertEqual(data['local_address_check'], 'BLOCKED')


if __name__ == '__main__':
    unittest.main()
