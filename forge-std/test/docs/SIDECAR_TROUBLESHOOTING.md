# X3 Sidecar Troubleshooting Guide

This guide provides solutions for common issues encountered when operating the X3 sidecar.

## Table of Contents

1. [Startup and Initialization Issues](#startup-and-initialization-issues)
2. [Database Connection Problems](#database-connection-problems)
3. [Gateway Communication Failures](#gateway-communication-failures)
4. [Authentication and Authorization Errors](#authentication-and-authorization-errors)
5. [Performance and Timeout Issues](#performance-and-timeout-issues)
6. [Resource and Capacity Issues](#resource-and-capacity-issues)
7. [Debugging Tools and Techniques](#debugging-tools-and-techniques)
8. [Recovery Procedures](#recovery-procedures)

## Startup and Initialization Issues

### Issue: Sidecar fails to start - "Database connection failed"

**Symptoms:**
```
Error: failed to connect to database
Error: cannot initialize schema
Application exiting
```

**Root Causes:**
- Database URL is incorrect
- PostgreSQL service is not running
- Database user credentials are wrong
- Network connectivity to database is blocked
- PostgreSQL port is not exposed

**Solutions:**

1. **Verify database URL format:**
   ```bash
   # Check environment variable
   echo $X3_DATABASE_URL
   
   # Expected format
   # postgresql://user:password@host:port/database
   ```

2. **Test database connectivity:**
   ```bash
   # Using psql
   psql -U x3_user -h localhost -d x3_sidecar -c "SELECT 1;"
   
   # If using Docker
   docker exec x3-postgres psql -U x3_user -d x3_sidecar -c "SELECT 1;"
   ```

3. **Verify database exists:**
   ```bash
   psql -U postgres -h localhost -c "SELECT datname FROM pg_database WHERE datname='x3_sidecar';"
   ```

4. **Check PostgreSQL is running:**
   ```bash
   # Docker
   docker ps | grep postgres
   
   # Kubernetes
   kubectl get pods -l app=postgres
   
   # System service
   systemctl status postgresql
   ```

5. **Verify network connectivity:**
   ```bash
   # From sidecar container/host
   nc -zv database-host 5432
   
   # Using psql with verbose
   psql -v ON_ERROR_STOP=on -U x3_user -h database-host -d x3_sidecar -c "SELECT version();"
   ```

### Issue: "Schema initialization error" on first run

**Symptoms:**
```
Error: failed to create tables
Error: permission denied for schema public
```

**Root Causes:**
- Database user lacks CREATE TABLE permissions
- Schema already exists with incompatible tables
- Insufficient disk space

**Solutions:**

1. **Grant required permissions:**
   ```bash
   # Connect as postgres superuser
   psql -U postgres -d x3_sidecar -c "
   GRANT CREATE ON SCHEMA public TO x3_user;
   GRANT USAGE ON SCHEMA public TO x3_user;
   GRANT CREATE ON DATABASE x3_sidecar TO x3_user;
   "
   ```

2. **Check disk space:**
   ```bash
   # Host
   df -h /var/lib/postgresql
   
   # Docker container
   docker exec x3-postgres df -h /var/lib/postgresql/data
   
   # Kubernetes PVC
   kubectl get pvc -n x3
   ```

3. **Verify no conflicting tables:**
   ```bash
   psql -U x3_user -d x3_sidecar -c "\dt"
   ```

### Issue: "Unable to bind to port 9090"

**Symptoms:**
```
Error: failed to bind to 0.0.0.0:9090
Error: port already in use
```

**Solutions:**

1. **Check what's using the port:**
   ```bash
   lsof -i :9090
   netstat -tlnp | grep 9090
   ```

2. **Kill the process using the port:**
   ```bash
   kill -9 <PID>
   ```

3. **Use different port:**
   ```bash
   X3_HEALTH_PORT=9091 ./x3-sidecar
   ```

## Database Connection Problems

### Issue: Connection pool exhausted - "no more connections available"

**Symptoms:**
```
Error: unable to get connection from pool
Error: connection pool exhausted
Submissions failing with connection timeout
```

**Root Causes:**
- Too many concurrent requests
- Connections not being released properly
- Database connection limit reached
- Long-running queries holding connections

**Solutions:**

1. **Check active connections:**
   ```bash
   psql -U x3_user -d x3_sidecar -c "
   SELECT count(*) as active_connections 
   FROM pg_stat_activity 
   WHERE datname='x3_sidecar';
   "
   ```

2. **Increase connection pool size:**
   ```bash
   # In configuration or environment
   X3_DATABASE_MAX_CONNECTIONS=50
   
   # Also check PostgreSQL max_connections
   psql -U postgres -c "SHOW max_connections;"
   
   # Increase if needed
   psql -U postgres -c "ALTER SYSTEM SET max_connections = 100;"
   psql -U postgres -c "SELECT pg_reload_conf();"
   ```

3. **Check for long-running queries:**
   ```bash
   psql -U x3_user -d x3_sidecar -c "
   SELECT pid, query, state, query_start 
   FROM pg_stat_activity 
   WHERE state != 'idle' 
   ORDER BY query_start;
   "
   ```

4. **Monitor connection usage:**
   ```bash
   # Watch in real-time (every 2 seconds)
   watch -n 2 "psql -U x3_user -d x3_sidecar -c 'SELECT count(*) FROM pg_stat_activity;'"
   ```

### Issue: "Database connection timeout"

**Symptoms:**
```
Error: connection timeout after 5000ms
Error: failed to establish connection
```

**Root Causes:**
- Database is slow or overloaded
- Network latency is high
- Connection timeout is too short
- Firewall rules blocking connection

**Solutions:**

1. **Increase connection timeout:**
   ```bash
   X3_DATABASE_CONNECTION_TIMEOUT_MS=15000
   ```

2. **Check database health:**
   ```bash
   psql -U postgres -c "SELECT version();"
   
   # Check PostgreSQL logs
   docker logs x3-postgres --tail 50
   ```

3. **Check network latency:**
   ```bash
   ping -c 5 database-host
   traceroute database-host
   ```

4. **Verify firewall rules:**
   ```bash
   # From sidecar machine
   telnet database-host 5432
   ```

## Gateway Communication Failures

### Issue: "Unable to reach gateway" - gateway connection errors

**Symptoms:**
```
Error: failed to connect to gateway
Error: gateway unreachable
Retries exhausted after 3 attempts
```

**Root Causes:**
- Gateway service is down
- Gateway URL is incorrect
- Network connectivity is blocked
- Firewall rules preventing connection
- TLS certificate issues

**Solutions:**

1. **Verify gateway URL:**
   ```bash
   echo $X3_GATEWAY_URL
   # Should be: http://gateway-service:8080 or https://gateway.example.com
   ```

2. **Test gateway connectivity:**
   ```bash
   # Using curl
   curl -v http://$X3_GATEWAY_URL/health
   
   # From Docker container
   docker exec x3-sidecar curl -v http://x3-gateway:8080/health
   
   # From Kubernetes pod
   kubectl exec -n x3 x3-sidecar-pod -- curl -v http://x3-gateway:8080/health
   ```

3. **Check gateway is running:**
   ```bash
   # Docker
   docker ps | grep gateway
   docker logs x3-gateway --tail 50
   
   # Kubernetes
   kubectl get pods -n x3 -l app=x3-gateway
   kubectl logs -n x3 -l app=x3-gateway
   ```

4. **Verify network connectivity:**
   ```bash
   # Test DNS resolution
   nslookup gateway-service
   dig gateway-service
   
   # Test network reachability
   nc -zv gateway-service 8080
   telnet gateway-service 8080
   ```

5. **Check firewall rules:**
   ```bash
   # On gateway host
   sudo iptables -L -n | grep 8080
   
   # Or check with netstat
   netstat -tlnp | grep 8080
   ```

6. **If using TLS, verify certificates:**
   ```bash
   # Check certificate validity
   openssl s_client -connect gateway-service:8443 -showcerts
   
   # Check certificate expiration
   openssl x509 -noout -dates -in /path/to/cert.pem
   ```

### Issue: "401 Unauthorized" - authentication failure with gateway

**Symptoms:**
```
Error: HTTP 401 Unauthorized
Error: Invalid authentication credentials
Sidecar not retrying (auth errors don't trigger retry)
```

**Root Causes:**
- API key/token is missing or incorrect
- Credentials have expired
- Wrong authentication header format
- Gateway authentication configuration changed

**Solutions:**

1. **Verify API key is set:**
   ```bash
   echo $X3_API_KEY
   echo $X3_GATEWAY_AUTH_TOKEN
   ```

2. **Check API key format:**
   ```bash
   # Verify key is not empty
   [ -z "$X3_API_KEY" ] && echo "Key is empty" || echo "Key is set"
   
   # Check length (typically 32+ characters)
   echo $X3_API_KEY | wc -c
   ```

3. **Verify authentication header:**
   ```bash
   # Test with correct auth
   curl -v \
     -H "Authorization: Bearer $X3_API_KEY" \
     http://$X3_GATEWAY_URL/api/v1/benchmarks/results
   ```

4. **Regenerate credentials if needed:**
   - Contact gateway administrator
   - Rotate API keys: typically every 90 days
   - Update sidecar configuration with new credentials
   - Verify new credentials work before deploying

5. **Check gateway auth configuration:**
   ```bash
   # Request auth configuration details from gateway admin
   # Common issues: auth method changed, header name changed, token expired
   ```

### Issue: "502 Bad Gateway" or "503 Service Unavailable"

**Symptoms:**
```
Error: HTTP 502 Bad Gateway
Error: HTTP 503 Service Unavailable
Retries occur but eventually fail
```

**Root Causes:**
- Gateway is overloaded
- Gateway backend service is down
- Gateway is restarting
- Rate limiting triggered

**Solutions:**

1. **Check gateway health:**
   ```bash
   curl -I http://$X3_GATEWAY_URL/health
   ```

2. **Check gateway load:**
   ```bash
   # Request gateway metrics
   curl http://$X3_GATEWAY_URL/metrics | grep gateway_requests
   ```

3. **Adjust retry configuration:**
   ```bash
   X3_MAX_RETRIES=5
   X3_RETRY_BACKOFF_MS=2000
   X3_REQUEST_TIMEOUT_MS=60000
   ```

4. **Check rate limits:**
   - Look for rate limit headers in response: `X-RateLimit-Remaining`
   - Reduce batch size: `X3_BATCH_SIZE=250`
   - Increase time between submissions

5. **Monitor gateway logs:**
   ```bash
   docker logs x3-gateway --follow
   kubectl logs -n x3 -l app=x3-gateway --follow
   ```

## Authentication and Authorization Errors

### Issue: "Permission denied" or "403 Forbidden"

**Symptoms:**
```
Error: HTTP 403 Forbidden
Error: insufficient permissions
Submissions rejected by gateway
```

**Root Causes:**
- API key has insufficient scopes
- Sidecar account doesn't have required role
- Benchmark category not authorized
- Tenant/organization mismatch

**Solutions:**

1. **Verify API key scopes:**
   ```bash
   # Contact gateway administrator to confirm scopes
   # Typical scopes needed: write:benchmarks, read:benchmarks
   ```

2. **Check sidecar account permissions:**
   ```bash
   # Request permission details from gateway admin
   # Common permissions: submit_benchmarks, read_results, manage_jobs
   ```

3. **Verify tenant/organization:**
   ```bash
   echo $X3_TENANT_ID
   echo $X3_ORGANIZATION_ID
   
   # Ensure these match gateway configuration
   ```

4. **Request permission upgrade:**
   - Contact gateway administrator
   - Provide sidecar service account details
   - Specify required permissions
   - Verify after upgrade

## Performance and Timeout Issues

### Issue: "Request timeout" - submissions timing out

**Symptoms:**
```
Error: request timeout after 30000ms
Error: no response from gateway
Submissions fail intermittently
```

**Root Causes:**
- Timeout is too short for submission size
- Network latency is high
- Gateway is processing slowly
- Batch size too large

**Solutions:**

1. **Increase request timeout:**
   ```bash
   X3_REQUEST_TIMEOUT_MS=60000  # 60 seconds
   ```

2. **Check network latency:**
   ```bash
   ping -c 10 $X3_GATEWAY_URL
   # Look for latency > 100ms as indication of issues
   ```

3. **Reduce batch size:**
   ```bash
   X3_BATCH_SIZE=100  # Default 500, try smaller
   ```

4. **Check gateway performance:**
   ```bash
   # Measure gateway response time
   time curl -X GET http://$X3_GATEWAY_URL/health
   
   # If > 5 seconds, gateway is slow
   ```

5. **Monitor submission size:**
   - Reduce benchmark report detail if unnecessary
   - Compress benchmark data
   - Consider splitting large reports

### Issue: "Batch timeout" - submissions accumulating without being sent

**Symptoms:**
```
Submissions pending but not being sent
Logs show batch timeout errors
Submissions eventually succeed but delayed
```

**Root Causes:**
- Batch timeout too short
- Batch size never reached
- Low submission rate
- Gateway rejections causing retries

**Solutions:**

1. **Increase batch timeout:**
   ```bash
   X3_BATCH_TIMEOUT_MS=15000  # 15 seconds
   ```

2. **Check submission rate:**
   ```bash
   # Monitor logs for submission frequency
   docker logs x3-sidecar | grep "submission_received" | wc -l
   ```

3. **Reduce batch size if submission rate is low:**
   ```bash
   X3_BATCH_SIZE=50  # Smaller batches send quicker
   ```

4. **Monitor batch accumulation:**
   ```bash
   # Check database for pending submissions
   psql -U x3_user -d x3_sidecar -c "
   SELECT status, COUNT(*) 
   FROM submissions 
   GROUP BY status;
   "
   ```

## Resource and Capacity Issues

### Issue: "Out of memory" - sidecar crashes with OOM

**Symptoms:**
```
Error: out of memory
Process killed: signal: SIGKILL
Docker: OOMKilled
Kubernetes: OOMKilled
```

**Root Causes:**
- Batch size too large
- Too many concurrent submissions
- Memory leak in sidecar code
- Insufficient memory allocated

**Solutions:**

1. **Increase memory allocation:**
   ```bash
   # Docker
   docker run -m 512m x3-sidecar
   
   # Kubernetes
   # Increase in deployment: limits.memory: "512Mi"
   
   # Local
   # Allocate more system RAM
   ```

2. **Reduce batch size:**
   ```bash
   X3_BATCH_SIZE=100  # Smaller batches use less memory
   ```

3. **Monitor memory usage:**
   ```bash
   # Docker
   docker stats x3-sidecar --no-stream
   
   # Kubernetes
   kubectl top pod -n x3 x3-sidecar-pod
   
   # Local
   ps aux | grep x3-sidecar
   ```

4. **Enable memory limits:**
   ```yaml
   # Kubernetes
   resources:
     limits:
       memory: "512Mi"
     requests:
       memory: "256Mi"
   ```

### Issue: "Disk space full" - sidecar unable to write to database

**Symptoms:**
```
Error: disk full
Error: no space left on device
Unable to write to database
```

**Root Causes:**
- Database data directory full
- Log files consuming space
- Temporary files not cleaned up
- Database backups filling disk

**Solutions:**

1. **Check disk usage:**
   ```bash
   df -h /var/lib/postgresql
   du -sh /var/lib/postgresql/*
   ```

2. **Clean up old logs:**
   ```bash
   # Find log files
   find /var/log -name "*x3-sidecar*" -type f
   
   # Rotate and remove old logs
   find /var/log -name "*x3-sidecar*" -mtime +30 -delete
   ```

3. **Check database size:**
   ```bash
   psql -U x3_user -d x3_sidecar -c "
   SELECT pg_size_pretty(pg_database_size('x3_sidecar'));
   "
   ```

4. **Clean up temporary files:**
   ```bash
   rm -rf /tmp/x3-*
   ```

5. **Expand disk if permanent storage needed:**
   - Add additional disk space
   - Mount new volume to database directory
   - Archive old submissions if retention allows

## Debugging Tools and Techniques

### Enable Debug Logging
```bash
# Set log level to DEBUG
X3_LOG_LEVEL=DEBUG ./x3-sidecar

# For Docker
docker run -e X3_LOG_LEVEL=DEBUG x3-sidecar

# For Kubernetes
kubectl set env deployment/x3-sidecar X3_LOG_LEVEL=DEBUG -n x3
```

### View Logs
```bash
# Local
tail -f /var/log/x3-sidecar.log

# Docker
docker logs -f x3-sidecar

# Docker with filtering
docker logs x3-sidecar | grep ERROR

# Kubernetes
kubectl logs -f x3-sidecar -n x3
kubectl logs -f x3-sidecar -n x3 | grep ERROR
```

### Database Debugging
```bash
# Connect to database
psql -U x3_user -d x3_sidecar

# View table structure
\dt

# Check submissions table
SELECT COUNT(*), status FROM submissions GROUP BY status;

# Check failed submissions details
SELECT id, status, error_message, created_at 
FROM submissions 
WHERE status='failed' 
ORDER BY created_at DESC 
LIMIT 10;

# Check pending submissions age
SELECT id, created_at, NOW() - created_at as age 
FROM submissions 
WHERE status='pending' 
ORDER BY created_at 
LIMIT 10;
```

### Network Debugging
```bash
# Test gateway connectivity with verbose output
curl -v http://$X3_GATEWAY_URL/health

# Test with connection timeout
curl --connect-timeout 5 http://$X3_GATEWAY_URL/health

# Capture HTTP traffic (if inside container)
docker exec x3-sidecar tcpdump -i eth0 -n -A | grep -E "POST|GET|HTTP"
```

### Metrics Inspection
```bash
# View prometheus metrics
curl http://localhost:9090/metrics

# Filter for specific metrics
curl http://localhost:9090/metrics | grep x3_sidecar

# Watch metrics in real-time
watch -n 5 "curl -s http://localhost:9090/metrics | grep x3_sidecar_submissions"
```

## Recovery Procedures

### Complete Service Reset
```bash
# Stop sidecar
docker stop x3-sidecar

# Clear pending submissions (WARNING: data loss)
psql -U x3_user -d x3_sidecar -c "DELETE FROM submissions WHERE status='pending';"

# Restart sidecar
docker start x3-sidecar

# Verify health
curl http://localhost:9090/health
```

### Database Recovery
```bash
# Backup database before recovery
pg_dump -U x3_user x3_sidecar > backup.sql

# Check for corrupted tables
psql -U x3_user -d x3_sidecar -c "REINDEX DATABASE x3_sidecar;"

# Analyze and optimize
psql -U x3_user -d x3_sidecar -c "ANALYZE;"

# Restart database
docker restart x3-postgres
```

### Retry Failed Submissions
```bash
# Mark failed submissions as pending for retry
psql -U x3_user -d x3_sidecar -c "
UPDATE submissions 
SET status='pending', retry_count=0 
WHERE status='failed' AND created_at > NOW() - INTERVAL '1 day';
"

# Restart sidecar to pick up retries
docker restart x3-sidecar
```

### Escalation Contacts

If issues persist after following these troubleshooting steps:

1. **Check sidecar logs for error codes**
2. **Document current state:**
   - Sidecar version
   - Gateway version
   - Database logs (last 50 lines)
   - Network configuration
   - Resource allocation (memory, CPU, disk)
3. **Contact support with:**
   - Detailed logs
   - Reproduction steps
   - Current system state
   - Recent configuration changes
