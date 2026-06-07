# gpu-swarm v0.1.0 release draft

**Highlights**
- Coordinator and Node binaries for Linux (x86_64)
- Quickstart, default configs, systemd service files
- Dockerfile + docker-compose for NVIDIA GPU runtime
- Unit & integration tests passing

**Assets**
- `gpu-swarm-linux-x86_64-v0.1.0.tar.gz` (includes binaries, configs, systemd units, README_QUICKSTART)
- `gpu-swarm-linux-x86_64-v0.1.0.tar.gz.sha256`

**Changelog**
- Initial release: core scheduler, verification, node registry, basic P2P/network stub, task types, simulated GPU support

**Publish steps (manual using GitHub CLI)**
1. Create draft release locally:
   gh release create v0.1.0 dist/gpu-swarm-linux-x86_64-v0.1.0.tar.gz --title "gpu-swarm v0.1.0" --notes-file RELEASE_NOTES/gpu-swarm-v0.1.0.md

2. Add checksum file as a secondary asset (if gh CLI supports):
   gh release upload v0.1.0 dist/gpu-swarm-linux-x86_64-v0.1.0.tar.gz.sha256

3. Mark as published (or keep as draft for review)

**Post-publish**
- Update website/docs with a download link and SHA256 checksum
- Add a short `docs/gpu-node-setup.md` link to the release notes

**Notes**
- If you want cross-arch builds (arm64), we can add CI (GitHub Actions) to produce images and binaries automatically, sign artifacts, and produce checksums.