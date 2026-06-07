# Explorer — Bare‑metal Deployment (no containers)

This guide covers deploying `apps/explorer` to bare‑metal servers without containers.

Overview
- Build on CI or locally, copy app directory to server, run `npm ci` and `npm run build` on the server, and run `npm run start` as a systemd service.
- Use Nginx as a reverse proxy (TLS termination and health checks).

Prerequisites on server
- Node 18+ installed
- Create user `explorer` and group `explorer`:
  sudo useradd -m -s /bin/bash explorer
- Install nginx and configure SSL (certs via certbot or your CA)

Quick deploy (manual)
1. Build locally and sync to server (example):
   ./deployment/baremetal/explorer/deploy.sh user@server.example.com /opt/explorer

2. Create systemd unit (copy `deployment/baremetal/explorer/systemd/explorer.service` to `/etc/systemd/system/explorer.service`), then:
   sudo systemctl daemon-reload
   sudo systemctl enable --now explorer.service

3. Configure nginx using `deployment/baremetal/explorer/nginx/explorer.conf` and reload nginx.

Health checks & monitoring
- Systemd: `journalctl -u explorer -f`
- Nginx reverse proxy health endpoint: `https://explorer.testnet.x3-chain.io/health`

Scaling (bare‑metal)
- For multiple nodes use a load balancer or keepalived + nginx in front of multiple explorer instances.
- Since containers are not used, scale by provisioning additional physical/VM servers and repeating the deploy; update your load‑balancer backend pool.

CI integration (recommended)
- Build artifacts in CI and use `rsync`/SSH to push release builds to bare‑metal servers (see `deployment/baremetal/explorer/deploy.sh`).
- CI should run smoke tests against the server and run rollback scripts if checks fail.

Notes
- Bare‑metal avoids container overhead but requires more ops effort (patching, runtime isolation, autoscaling manual). Use the Helm chart for Kubernetes when automated scaling / canary flows are required.

Contact: devops@x3-chain.io
