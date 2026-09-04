# Authentication & Auto-Start Setup Guide

Everything you need to know about the new authentication system and auto-start configuration for X3 Chain.

## Quick Start

### 1. **Install Systemd Services** (auto-start on boot)

```bash
sudo bash /home/lojak/Desktop/x3-chain-master/deployment/scripts/install-services.sh
```

This will:
- Create an "x3" system user
- Copy 3 service files to `/etc/systemd/system/`
- Enable and start services automatically

**Services installed:**
- `redis.service` - Cache & session store (port 6379)
- `ccgv-validator.service` - Cross-chain GPU validator (port 8000)
- `x3-intelligence.service` - Dashboard with authentication (port 5173)

### 2. **Create Configuration File**

```bash
cp deployment/env/.env.template .env
# Edit .env with your actual API keys and secrets
nano .env
```

**Critical environment variables:**
```
AUTH_SALT=your-secret-here                    # Used for password hashing
JWT_SECRET=your-jwt-secret                    # Used for JWT tokens
REDIS_HOST=localhost                          # Session storage
CCGV_RPC_ETHEREUM=https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY
CCGV_RPC_SOLANA=https://api.mainnet-beta.solana.com
```

### 3. **Change Default Credentials**

> ⚠️ **CRITICAL for production!** The demo credentials below are public and insecure.

**Current default login:**
- Username: `admin`
- Password: `x3-chain-2026`

**To change credentials:**

Edit `/apps/x3-intelligence/src/auth.ts`:

```typescript
// Find the login validation section (around line 23)
const DEFAULT_USERS = [
  {
    id: '1',
    username: 'admin',
    password: hashPassword('YOUR-NEW-PASSWORD'),  // Change this
    email: 'admin@example.com'
  }
];

// Also update AUTH_SALT for better security
const AUTH_SALT = process.env.AUTH_SALT || 'your-secure-random-salt';
```

Then rebuild the dashboard:
```bash
cd /home/lojak/Desktop/x3-chain-master/apps/x3-intelligence
npm install
npm run build
```

## Architecture

### Authentication Flow

```
User Login (LoginPage.tsx)
    ↓
authService.login() - POST /api/auth/login
    ↓
Backend validates credentials (auth.ts)
    ↓
JWT token issued (24hr expiry)
    ↓
Token stored in localStorage
    ↓
ProtectedRoute checks authentication
    ↓
Dashboard accessible
```

### Components & Files

#### **Frontend**
- **LoginPage.tsx** - Beautiful login UI with credential entry
- **AppBar.tsx** - Navigation bar with user menu & logout
- **ProtectedRoute.tsx** - Route wrapper requiring authentication
- **useAuth.ts** - React hook for auth state management
- **authService.ts** - API communication & token management

#### **Backend**
- **auth.ts** - Authentication endpoints & user validation
- **env template** - Configuration variables

#### **Infrastructure**
- **ccgv-validator.service** - Systemd service (auto-start)
- **x3-intelligence.service** - Systemd service (auto-start)
- **redis.service** - Systemd service (auto-start)
- **install-services.sh** - Service installer script
- **startup.sh** - Manual startup fallback

## How to Use

### Login

1. Navigate to http://localhost:5173
2. Enter credentials:
   - Username: `admin`
   - Password: `x3-chain-2026`
3. Click "Sign In"
4. You'll be redirected to the dashboard

### Logout

1. Click your username in the top-right corner
2. Select "Logout"
3. You'll be returned to the login page

### Change Password

1. Click your username in the top-right corner
2. Select "Change Password"
3. Enter current and new password
4. Confirm to update

## Integration with Your App

### Using the `useAuth` Hook

```typescript
import { useAuth } from '@/hooks/useAuth';

function MyComponent() {
  const { user, isAuthenticated, logout, login } = useAuth();
  
  return (
    <div>
      {isAuthenticated && <p>Welcome, {user?.username}!</p>}
      <button onClick={logout}>Logout</button>
    </div>
  );
}
```

### Protecting Routes

```typescript
import { ProtectedRoute } from '@/components/ProtectedRoute';
import Dashboard from '@/pages/Dashboard';

<Routes>
  <Route path="/login" element={<LoginPage />} />
  <Route 
    path="/dashboard" 
    element={
      <ProtectedRoute>
        <Dashboard />
      </ProtectedRoute>
    } 
  />
</Routes>
```

### Making Authenticated API Calls

```typescript
const { getAuthHeader } = useAuth();

const fetchDashboardData = async () => {
  const response = await fetch('/api/dashboard', {
    headers: getAuthHeader()
  });
  const data = await response.json();
  return data;
};
```

## Service Management

### Check Service Status

```bash
# Check if services are running
sudo systemctl status redis.service
sudo systemctl status ccgv-validator.service
sudo systemctl status x3-intelligence.service

# View logs for a service
sudo journalctl -u x3-intelligence.service -n 50
sudo journalctl -u ccgv-validator.service -n 50
sudo journalctl -u redis.service -n 50

# Follow logs in real-time
sudo journalctl -u x3-intelligence.service -f
```

### Start/Stop Services Manually

```bash
# Start individual service
sudo systemctl start x3-intelligence.service

# Stop individual service
sudo systemctl stop x3-intelligence.service

# Restart service
sudo systemctl restart x3-intelligence.service

# Enable/disable auto-start
sudo systemctl enable x3-intelligence.service
sudo systemctl disable x3-intelligence.service
```

### Fallback: Manual Startup

If systemd services aren't working, use the startup script:

```bash
bash deployment/scripts/startup.sh
```

This will:
1. Start Redis locally on port 6379
2. Start CCGV validator on port 8000
3. Start X3 Intelligence on port 5173

## Testing Authentication

### Manual Login Test

```bash
curl -X POST http://localhost:5173/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"x3-chain-2026"}'
```

Expected response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user": {
    "id": "1",
    "username": "admin",
    "email": "admin@example.com"
  },
  "expiresIn": 86400
}
```

### Validate Token

```bash
curl -X GET http://localhost:5173/api/auth/validate \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## Token Validation

Tokens are automatically validated:
- **On login:** Initial token validation
- **On protected routes:** Client-side expiry check
- **On API calls:** Server-side token verification
- **On refresh:** Extended session (+24 hours)

## Sessions & Storage

- **Frontend:** Tokens stored in `localStorage`
- **Backend:** Session data in memory (or Redis if configured)
- **Expiry:** 24 hours from login
- **Auto-refresh:** Can be extended via `/api/auth/refresh` endpoint

## Security Features

✅ JWT tokens with 24-hour expiry
✅ Password hashing with SHA256 + salt
✅ CORS headers on all responses
✅ HTTP-only cookie support (future)
✅ Token validation on every protected request
✅ Secure logout that clears all session data

## Troubleshooting

### "Connection failed. Is the server running?"

```bash
# Check if backend is running
curl http://localhost:8000

# Check if port is in use
sudo lsof -i :8000

# Start services manually
bash deployment/scripts/startup.sh
```

### "Invalid credentials" on known-good password

- Check .env file AUTH_SALT matches backend
- Verify default credentials weren't changed
- Look at backend logs: `sudo journalctl -u ccgv-validator -f`

### Default credentials aren't working

- Verify the admin user exists in auth.ts
- Check that password hashing is consistent
- Try resetting with the startup script

### Services won't start on boot

```bash
# Verify services are enabled
sudo systemctl list-unit-files | grep x3

# Enable a service
sudo systemctl enable x3-intelligence.service

# Check systemd journal for errors
sudo journalctl -xe
```

## Production Checklist

Before deploying to production:

- [ ] Change default admin password
- [ ] Set strong AUTH_SALT in .env
- [ ] Set strong JWT_SECRET in .env
- [ ] Configure actual RPC endpoints in .env
- [ ] Update REDIS_HOST if using external Redis
- [ ] Enable HTTPS for login page
- [ ] Set up Redis persistence (appendonly)
- [ ] Configure log rotation for /var/log/x3
- [ ] Test failover procedures
- [ ] Run security audit on password storage
- [ ] Set up monitoring for service health

## Architecture Diagram

```
┌─────────────────────────────────────────────┐
│         User Browser (localhost:5173)        │
│  ┌───────────────────────────────────────┐  │
│  │     LoginPage.tsx (Public)             │  │
│  │  - Username/password input             │  │
│  │  - Credentials validation              │  │
│  >>> Calls authService.login()            │  │
│  └────────────────┬──────────────────────┘  │
└─────────────────┼─────────────────────────┘
                  │ POST /api/auth/login
    ┌─────────────▼────────────────┐
    │  Backend (Port 8000)         │
    │  ┌──────────────────────┐    │
    │  │  auth.ts             │    │
    │  │ - User validation    │    │
    │  │ - Password check     │    │
    │  │ - JWT generation     │    │
    │  └──────────┬───────────┘    │
    │             │                │
    │  ┌──────────▼───────────┐    │
    │  │ Redis (Port 6379)    │    │
    │  │ - Session storage    │    │
    │  │ - Token cache        │    │
    │  └──────────────────────┘    │
    └─────────────────────────────┘
                  │ JWT Token
    ┌─────────────▼────────────────────┐
    │   Frontend (Protected Routes)     │
    │   ┌────────────────────────────┐  │
    │   │ ProtectedRoute.tsx          │  │
    │   │ - Check localStorage token  │  │
    │   │ - Validate with server      │  │
    │   │ - Redirect to login if fail │  │
    │   └────────────┬─────────────────┘  │
    │                │                    │
    │   ┌────────────▼────────────────┐  │
    │   │ AppBar.tsx (Navigation)     │  │
    │   │ - User menu                 │  │
    │   │ - Logout button             │  │
    │   │ - Settings links            │  │
    │   └─────────────────────────────┘  │
    │                                    │
    │   ┌─────────────────────────────┐  │
    │   │ Dashboard (Authenticated)   │  │
    │   │ - Shows user data           │  │
    │   │ - Real-time metrics         │  │
    │   │ - All protected content     │  │
    │   └─────────────────────────────┘  │
    └────────────────────────────────────┘

┌────────────────────────────────────────────┐
│  Systemd Services (Auto-start on boot)     │
│  ├─ redis.service                          │
│  ├─ ccgv-validator.service                 │
│  └─ x3-intelligence.service                │
└────────────────────────────────────────────┘
```

## File Structure

```
x3-chain-master/
├── apps/x3-intelligence/src/
│   ├── components/
│   │   ├── LoginPage.tsx          # Login UI
│   │   ├── LoginPage.css          # Login styling
│   │   ├── AppBar.tsx             # Navigation bar
│   │   ├── AppBar.css             # AppBar styling
│   │   └── ProtectedRoute.tsx     # Auth wrapper
│   ├── services/
│   │   └── authService.ts         # API communication
│   ├── hooks/
│   │   └── useAuth.ts             # Auth hook
│   └── auth.ts                    # Backend auth logic
│
├── deployment/
│   ├── systemd/
│   │   ├── redis.service
│   │   ├── ccgv-validator.service
│   │   └── x3-intelligence.service
│   ├── scripts/
│   │   ├── install-services.sh
│   │   └── startup.sh
│   └── env/
│       └── .env.template
│
└── cross-chain-gpu-validator/
    └── ... (validator code)
```

## Next Steps

1. ✅ **Install services:** `sudo bash deployment/scripts/install-services.sh`
2. ✅ **Configure environment:** Create and customize `.env`
3. ✅ **Change credentials:** Update password in `auth.ts`
4. ✅ **Test login:** Visit http://localhost:5173 after reboot
5. ✅ **Setup monitoring:** Configure log rotation and alerts
6. ✅ **Production deployment:** Follow production checklist

---

**Questions?** Check the logs:
```bash
sudo journalctl -u x3-intelligence.service -n 100 -f
```

**Need to reset?** Remove and reinstall services:
```bash
sudo bash deployment/scripts/install-services.sh  # Reinstalls everything
```
