# How To Recover Git Corruption Safely

This runbook captures the safe recovery flow used for severe local Git DB corruption in this repository.

## Scope

Use this when one or more of these symptoms appear:

- `fatal: not a git repository` in a valid checkout
- `index file smaller than expected`
- `bad ref` or `invalid sha1 pointer`
- `git fsck` reports pack checksum mismatches or corrupt loose objects
- `git push` fails due local ref/object corruption

## Safety-First Procedure

1. Freeze and preserve local state before edits:
- Save a full file manifest excluding `.git*`.
- Preserve raw metadata files: `.git/index`, `.git/HEAD`, `.git/config`, `.git/refs`, `.git/logs`.

2. Confirm corruption class:
- Run `git fsck --full --no-reflogs`.
- Check `.git/HEAD`, `.git/config`, and `git show-ref --heads`.
- If refs and objects are widely corrupt, do not try in-place object surgery.

3. Rebuild Git metadata in-place (safe path):
- Move broken `.git` aside (forensics only).
- `git init`
- Re-add remote and identity settings.
- `git fetch origin main`
- `git update-ref refs/heads/main FETCH_HEAD`
- `git symbolic-ref HEAD refs/heads/main`
- `git reset --mixed FETCH_HEAD`
- Create a recovery branch for PR workflow.

4. Prevent accidental backup commits:
- Add ignore rules for `.git.corrupt-*` and recovery snapshot directories.
- Remove any accidentally tracked backup artifacts from index.

5. Commit and push recovery branch:
- `git add -A`
- Commit with explicit recovery message.
- Push branch to origin and open PR to `main`.

## Operator Checklist

- [ ] Local file manifest captured
- [ ] Raw `.git` metadata copied for forensics
- [ ] Corruption confirmed by `git fsck`
- [ ] Fresh `.git` metadata initialized and attached to `origin/main`
- [ ] Recovery branch created
- [ ] Backup artifact paths ignored in `.gitignore`
- [ ] Recovery branch pushed to origin
- [ ] PR opened against `main`
- [ ] Build/tests executed or explicitly triaged if blocked

## Notes

- Prefer PR-based merge. Do not force-push to `main`.
- If GitHub rejects push due large files, remove backup artifacts from tracked content and amend the commit.
- Keep one minimal forensic snapshot, not multiple multi-GB corrupted object stores.
