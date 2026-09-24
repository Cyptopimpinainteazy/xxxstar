"""Make the GPU-validator package importable for the root copies of its tests.

These tests import `cross_chain_gpu_validator.metrics` and friends, but the
package lives under `infra-structure/validator/src/`, which is not on
`sys.path` when pytest is invoked from the repository root. Without this the
whole module fails to import and the suite reports a collection error instead of
a result — the tests were never actually absent, just unreachable.
"""

import sys
from pathlib import Path

PACKAGE_SRC = Path(__file__).resolve().parents[2] / "infra-structure" / "validator" / "src"

if PACKAGE_SRC.is_dir() and str(PACKAGE_SRC) not in sys.path:
    sys.path.insert(0, str(PACKAGE_SRC))
