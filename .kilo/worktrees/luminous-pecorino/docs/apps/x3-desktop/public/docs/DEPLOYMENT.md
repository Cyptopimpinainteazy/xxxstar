# Deployment Guide

## Local Development

### Prerequisites
- Node.js 16+
- npm or yarn

### Setup

```bash
cd apps/swarm-dashboard
npm install
cp .env.example .env.local
```

### Development Server

```bash
npm run dev
```

The dashboard will start on `http://localhost:5173` with hot module reloading.

**Configure API endpoints** in `.env.local`:
```env
VITE_API_BASE_URL=http://localhost:5000/api
VITE_WS_BASE_URL=ws://localhost:5000/ws
```

## Production Build

### Build

```bash
npm run build
```

Output is in `dist/` directory. Ready for deployment.

### Preview

```bash
npm run preview
```

Preview the production build locally on `http://localhost:4173`

## Docker Deployment

### Build Image

```bash
docker build -t x3-chain-dashboard:latest .
```

### Run Container

```bash
docker run -p 3000:80 \
  -e VITE_API_BASE_URL=https://api.your-domain.com/api \
  -e VITE_WS_BASE_URL=wss://api.your-domain.com/ws \
  x3-chain-dashboard:latest
```

## Kubernetes Deployment

### Create ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: dashboard-config
data:
  VITE_API_BASE_URL: "https://api.your-domain.com/api"
  VITE_WS_BASE_URL: "wss://api.your-domain.com/ws"
```

### Create Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: gpu-swarm-dashboard
spec:
  replicas: 2
  selector:
    matchLabels:
      app: dashboard
  template:
    metadata:
      labels:
        app: dashboard
    spec:
      containers:
      - name: dashboard
        image: x3-chain-dashboard:latest
        ports:
        - containerPort: 80
        envFrom:
        - configMapRef:
            name: dashboard-config
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "500m"
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: dashboard-service
spec:
  selector:
    app: dashboard
  ports:
  - protocol: TCP
    port: 80
    targetPort: 80
  type: LoadBalancer
```

## Nginx Configuration

```nginx
server {
    listen 80;
    server_name dashboard.your-domain.com;

    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name dashboard.your-domain.com;

    ssl_certificate /etc/ssl/certs/your-cert.crt;
    ssl_certificate_key /etc/ssl/private/your-key.key;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    root /usr/share/nginx/html;
    index index.html;

    # SPA routing - serve index.html for non-file requests
    location / {
        try_files $uri $uri/ /index.html;
    }

    # Cache static assets
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
        expires 7d;
        add_header Cache-Control "public, immutable";
    }

    # API proxy (optional)
    location /api/ {
        proxy_pass http://api-backend:5000/api/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }

    # WebSocket proxy
    location /ws/ {
        proxy_pass ws://api-backend:5000/ws/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
```

## AWS S3 + CloudFront

### Upload to S3

```bash
aws s3 sync dist/ s3://your-bucket-name/dashboard/ --delete
```

### CloudFront Configuration

1. Create CloudFront distribution
2. Set origin domain to S3 bucket
3. Set default root object to `index.html`
4. Configure error pages (404 → `/index.html`)
5. Set cache behavior for SPA

### Example CloudFront Policy

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Service": "cloudfront.amazonaws.com"
      },
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::your-bucket-name/*"
    }
  ]
}
```

## Vercel Deployment

### Connect Repository

1. Push code to GitHub/GitLab/Bitbucket
2. Import project in Vercel dashboard
3. Configure environment variables

### Environment Variables
```
VITE_API_BASE_URL=https://your-api-domain.com/api
VITE_WS_BASE_URL=wss://your-api-domain.com/ws
```

### Build & Deploy

Vercel automatically builds and deploys on push to main branch.

## Monitoring & Performance

### Enable Monitoring

1. **Sentry** - error tracking:
```bash
npm install @sentry/react
```

2. **Google Analytics** - user analytics:
```bash
npm install @react-ga/core
```

3. **New Relic** - performance monitoring:
```bash
npm install @newrelic/browser-agent
```

### Performance Tips

- Enable gzip compression on server
- Use CDN for static assets
- Set appropriate cache headers
- Enable BROTLI compression
- Monitor Core Web Vitals

## Health Checks

The dashboard includes health check endpoint:

```bash
curl http://localhost:5173/
```

Returns 200 if healthy.

## Backup & Recovery

### Database
- Ensure API backend is backed up separately
- Dashboard is stateless, no local data to backup

### Configuration
- Back up `.env.local` environment files
- Store securely in secrets manager

### Disaster Recovery
1. Rebuild and redeploy from source
2. Restore environment configuration
3. Reconnect to API backend
