import json
import subprocess
import sys
from pathlib import Path


def test_runner_end_to_end():
    root = Path(__file__).resolve().parents[2]
    runner = root / 'x3-lang' / 'runner.py'
    example = root / 'x3-lang' / 'examples' / 'arb_solana_eth.x3'

    proc = subprocess.run([sys.executable, str(runner), str(example)], capture_output=True, check=True)
    result = json.loads(proc.stdout.decode())

    assert result.get('status') == 'ok'
    assert 'steps' in result
    assert 'emitted' in result
    assert isinstance(result['emitted'], dict)
    assert 'constraint_results' in result
    assert any(r['constraint'] == 'atomic' for r in result['constraint_results'])
    assert result['intent'] == 'arb_solana_eth'
