import express, { Request, Response, NextFunction } from 'express';
import session from 'express-session';
import { login, validateSession, logout, getSessionUser, initializeAuth, isAuthenticated } from './auth';

const router = express.Router();

// Initialize auth system
initializeAuth();

// Session middleware
router.use(session({
  secret: process.env.SESSION_SECRET || 'x3-chain-secret-key-change-in-production',
  resave: false,
  saveUninitialized: true,
  cookie: {
    secure: process.env.NODE_ENV === 'production', // HTTPS only in production
    httpOnly: true,
    maxAge: 24 * 60 * 60 * 1000 // 24 hours
  }
}));

// Auth middleware
export function authMiddleware(req: Request, res: Response, next: NextFunction) {
  const token = req.session?.authToken;
  
  if (!isAuthenticated(token)) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  
  next();
}

// Login endpoint
router.post('/api/auth/login', (req: Request, res: Response) => {
  const { username, password } = req.body;

  if (!username || !password) {
    return res.status(400).json({ error: 'Missing username or password' });
  }

  const sessionData = login(username, password);
  if (!sessionData) {
    return res.status(401).json({ error: 'Invalid credentials' });
  }

  req.session!.authToken = sessionData.token;
  req.session!.userId = sessionData.userId;

  return res.json({
    success: true,
    token: sessionData.token,
    expiresAt: sessionData.expiresAt
  });
});

// Logout endpoint
router.post('/api/auth/logout', (req: Request, res: Response) => {
  const token = req.session?.authToken;
  if (token) {
    logout(token);
  }
  req.session!.destroy(() => {});
  return res.json({ success: true });
});

// Check auth status
router.get('/api/auth/status', (req: Request, res: Response) => {
  const token = req.session?.authToken;
  
  if (!isAuthenticated(token)) {
    return res.json({ authenticated: false });
  }

  const user = getSessionUser(token!);
  return res.json({
    authenticated: true,
    username: user?.username
  });
});

// Protected dashboard endpoint
router.get('/api/dashboard', authMiddleware, (req: Request, res: Response) => {
  const token = req.session?.authToken;
  const user = getSessionUser(token!);
  
  return res.json({
    message: `Welcome ${user?.username}!`,
    timestamp: new Date().toISOString()
  });
});

export default router;
