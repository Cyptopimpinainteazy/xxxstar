Agent Scanner

Usage:

- Run a scan: python3 tools/agent_scanner/scan.py --repo .
- Outputs: tools/agent_scanner/checklist.json and CHECKLIST.md
- To delete selected ids: create a file ids.txt with one id per line and run:
  python3 tools/agent_scanner/scan.py --repo . --delete ids.txt --yes

Notes:
- Does not delete without confirmation unless --yes is passed.
- Duplicate item deletions keep the first file and remove other identical copies by content hash.
- Thresholds: stale files > 365 days (default), artifact dirs > 180 days.
- Adjust thresholds with --stale-days and --artifact-days.
