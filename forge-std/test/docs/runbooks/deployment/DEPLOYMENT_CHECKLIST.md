# Deployment Readiness Checklist

Complete this checklist to prepare X3 Chain for production deployment.

## Pre-Deployment

### Infrastructure Setup
- [ ] Install systemd services: `sudo bash deployment/scripts/install-services.sh`
- [ ] Verify services are enabled: `sudo systemctl list-unit-files | grep x3`
- [ ] Check all 3 services are active: `sudo systemctl status redis ccgv-validator x3-intelligence`
- [ ] Verify ports are listening: 5173 (dashboard), 8000 (validator), 6379 (redis)
- [ ] Create `.env` file from template
- [ ] Set all required environment variables in `.env`

### Security Configuration
- [ ] Change default admin password in `auth.ts`
- [ ] Generate random AUTH_SALT (min 32 chars): `openssl rand -base64 32`
- [ ] Generate random JWT_SECRET (min 32 chars): `openssl rand -base64 32`
- [ ] Update CCGV_RPC_* endpoints with production API keys
- [ ] Review and update REDIS_HOST if using external Redis
- [ ] Set ENVIRONMENT=production in `.env`

### Application Configuration
- [ ] Update app title/branding in `AppBar.tsx` if needed
- [ ] Update default email in `auth.ts`
- [ ] Configure log paths in service files (currently `/var/log/x3`)
- [ ] Set NODE_ENV=production in `x3-intelligence.service`
- [ ] Set RUST_LOG level for `ccgv-validator.service`

### Database & Storage
- [ ] Redis is configured for persistence (appendonly=yes)
- [ ] Redis password protection enabled (requirepass)
- [ ] Backup strategy defined for Redis data
- [ ] Check disk space for Redis and logs (min 10GB recommended)
- [ ] Set up Redis replication if high-availability required

### Monitoring & Logging
- [ ] Set up log rotation for `/var/log/x3/*.log`
- [ ] Configure centralized logging (CloudWatch, Datadog, etc.) if needed
- [ ] Set up service health monitoring (systemd notify)
- [ ] Configure alerts for service crashes
- [ ] Test log viewing: `sudo journalctl -u x3-intelligence.service -n 50`

### Networking & HTTPS
- [ ] Configure firewall rules (whitelist ports: 5173, 8000, 6379)
- [ ] Set up reverse proxy (nginx/Apache) for HTTPS termination
- [ ] Configure SSL certificate (self-signed for dev, Let's Encrypt for prod)
- [ ] Update API_BASE_URL if behind reverse proxy
- [ ] Test HTTPS endpoint and certificate validity
- [ ] Configure CORS headers if needed

### Performance & Optimization
- [ ] Run load tests on dashboard endpoints
- [ ] Optimize database queries if needed
- [ ] Set up caching headers for static assets
- [ ] Configure gzip compression for responses
- [ ] Test with expected user load

## Testing

### Functional Testing
- First bootup behavior:
  - [ ] Reboot system
  - [ ] Verify all services start automatically
  - [ ] Services are ready within 30 seconds
  - [ ] Dashboard accessible at https://localhost:5173
  
- Authentication flow:
  - [ ] Login with new credentials works
  - [ ] Invalid credentials rejected
  - [ ] Session persists across page reload
  - [ ] Logout clears session
  - [ ] Expired tokens refreshed automatically
  
- Dashboard functionality:
  - [ ] All metrics display correctly
  - [ ] Real-time updates working
  - [ ] No console errors or warnings
  - [ ] Response times acceptable (<500ms)

### Security Testing
- [ ] XSS payloads blocked on input fields
- [ ] CSRF token validation working
- [ ] Password hashing verified (never plaintext)
- [ ] Tokens don't contain sensitive data
- [ ] Authorization enforced on all endpoints
- [ ] API responses don't leak sensitive info
- [ ] CORS properly restricted

### Reliability Testing
- [ ] Service crash recovery (systemd restart)
- [ ] Database connection resilience
- [ ] RPC endpoint failover (if multiple endpoints)
- [ ] Memory usage stable over 24 hours
- [ ] No log file size explosion
- [ ] Graceful shutdown/startup

### Integration Testing
- [ ] Cross-chain message processing works
- [ ] GPU validator receives correct transactions
- [ ] Dashboard metrics match validator state
- [ ] RPC calls complete successfully
- [ ] Error handling for rate limits

## Documentation

### User Documentation
- [ ] Update README with production notes
- [ ] Document admin password reset procedure
- [ ] Create user guide for dashboard features
- [ ] Document how to add new users (if applicable)
- [ ] Create troubleshooting guide

### Operational Documentation
- [ ] Document service dependencies (redis → validator → dashboard)
- [ ] Create service restart procedure
- [ ] Document log locations and retention
- [ ] Create backup/restore procedures
- [ ] Document emergency fallback (startup.sh)

### Security Documentation
- [ ] Document password policy requirements
- [ ] Create incident response procedure
- [ ] Document data retention policies
- [ ] List of sensitive configuration variables
- [ ] Access control documentation

## Deployment

### Pre-Deployment Preparation
- [ ] All changes committed and merged to main
- [ ] Version tags created for release
- [ ] Release notes prepared
- [ ] Rollback plan documented
- [ ] Data backup taken before deployment

### Deployment Steps
1. [ ] Pull latest code: `git pull origin main`
2. [ ] Run install script: `sudo bash deployment/scripts/install-services.sh`
3. [ ] Verify `.env` file is in place with correct values
4. [ ] Start services: `sudo systemctl start redis ccgv-validator x3-intelligence`
5. [ ] Monitor startup logs: `sudo journalctl -f`
6. [ ] Wait 30 seconds for full startup
7. [ ] Test dashboard: curl https://localhost:5173/api/health
8. [ ] Test login: credentials from `.env` AUTH_USER

### Post-Deployment Verification
- [ ] All services running: `sudo systemctl status *`
- [ ] Dashboard responsive: http://localhost:5173
- [ ] Login functional with current credentials
- [ ] Metrics updating in real-time
- [ ] No error messages in logs
- [ ] System memory usage normal (<50%)
- [ ] Disk usage normal (<80% full)

## Scaling Preparation

### Horizontal Scaling
- [ ] Redis configured for replication (master/slave)
- [ ] Validator can handle multiple instances
- [ ] Load balancing configuration for multiple dashboards
- [ ] Session state externalized to Redis

### Vertical Scaling
- [ ] Application tested with increased load
- [ ] Database query optimization completed
- [ ] Caching strategy implemented
- [ ] Memory limits set appropriately

## Maintenance

### Regular Tasks
- [ ] Weekly: Check logs for errors
- [ ] Biweekly: Review resource usage
- [ ] Monthly: Security updates and patches
- [ ] Quarterly: Full system audit
- [ ] Annually: Disaster recovery drill

### Backup & Recovery
- [ ] Daily backups of critical data
- [ ] Weekly full system snapshots
- [ ] Recovery time objective (RTO) defined: _____ minutes
- [ ] Recovery point objective (RPO) defined: _____ minutes
- [ ] Backup verification tested monthly

### Monitoring & Alerting
- [ ] Alert on service down condition
- [ ] Alert on high memory usage (>80%)
- [ ] Alert on disk full condition (>95%)
- [ ] Alert on slow API responses (>1000ms)
- [ ] Alert on authentication failures (>10/5min)

## Sign-Off

Production deployment approved by:

**Deployer Name:** _____________________
**Date:** _____________________
**Time:** _____________________

**Verified By:** _____________________
**Date:** _____________________

## Rollback Procedure (if needed)

1. Stop services: `sudo systemctl stop x3-intelligence ccgv-validator redis`
2. Revert code: `git checkout <previous-commit>`
3. Restore backup: (if applicable)
4. Restart services: `sudo systemctl start redis ccgv-validator x3-intelligence`
5. Verify: Check logs and test authentication
6. Document: Why rollback was needed, what went wrong

---

## Quick Reference: Critical Commands

```bash
# Install/update services (run once)
sudo bash /home/lojak/Desktop/x3-chain-master/deployment/scripts/install-services.sh

# View status
sudo systemctl status x3-intelligence.service

# View recent logs
sudo journalctl -u x3-intelligence.service -n 100

# Follow logs in real-time
sudo journalctl -u x3-intelligence.service -f

# Restart service
sudo systemctl restart x3-intelligence.service

# Emergency manual startup
bash /home/lojak/Desktop/x3-chain-master/deployment/scripts/startup.sh

# Check if ports are listening
sudo netstat -tulpn | grep LISTEN

# View environment variables
grep -v '^#' /home/lojak/Desktop/x3-chain-master/.env
```

---

**Questions?** Review [/docs/runbooks/getting-started/AUTHENTICATION_SETUP.md](/docs/runbooks/getting-started/AUTHENTICATION_SETUP.md) and [/docs/runbooks/getting-started/QUICK_START.md](/docs/runbooks/getting-started/QUICK_START.md)

