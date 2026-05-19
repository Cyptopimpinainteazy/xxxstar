// Simple authentication system for x3-intelligence dashboard

interface User {
  id: string;
  username: string;
  passwordHash: string;
  createdAt: Date;
}

interface Session {
  userId: string;
  token: string;
  expiresAt: Date;
}

const crypto = require('crypto');

// Default credentials (change in production)
const DEFAULT_USER = {
  username: 'admin',
  password: 'x3-chain-2026'
};

let users: Map<string, User> = new Map();
let sessions: Map<string, Session> = new Map();

function hashPassword(password: string): string {
  return crypto
    .createHash('sha256')
    .update(password + process.env.AUTH_SALT || 'x3-default-salt')
    .digest('hex');
}

export function initializeAuth(): void {
  // Initialize default admin user
  const adminHash = hashPassword(DEFAULT_USER.password);
  users.set(DEFAULT_USER.username, {
    id: 'admin-1',
    username: DEFAULT_USER.username,
    passwordHash: adminHash,
    createdAt: new Date()
  });
  console.log('[AUTH] Initialized with default admin account');
  console.log('[AUTH] ⚠️  Change password in production: AUTH_SALT env var');
}

export function login(username: string, password: string): Session | null {
  const user = users.get(username);
  if (!user) {
    console.log(`[AUTH] Login failed: user not found (${username})`);
    return null;
  }

  const passwordHash = hashPassword(password);
  if (passwordHash !== user.passwordHash) {
    console.log(`[AUTH] Login failed: invalid password (${username})`);
    return null;
  }

  // Generate session token
  const token = crypto.randomBytes(32).toString('hex');
  const expiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000); // 24 hours
  
  const session: Session = {
    userId: user.id,
    token,
    expiresAt
  };

  sessions.set(token, session);
  console.log(`[AUTH] Login successful (${username})`);
  
  return session;
}

export function validateSession(token: string): boolean {
  const session = sessions.get(token);
  if (!session) return false;
  
  if (new Date() > session.expiresAt) {
    sessions.delete(token);
    return false;
  }
  
  return true;
}

export function logout(token: string): void {
  sessions.delete(token);
  console.log('[AUTH] Logout successful');
}

export function getSessionUser(token: string): User | null {
  const session = sessions.get(token);
  if (!session) return null;
  
  const user = Array.from(users.values()).find(u => u.id === session.userId);
  return user || null;
}

export function isAuthenticated(token: string | undefined): boolean {
  if (!token) return false;
  return validateSession(token);
}

// For testing/admin: add new user
export function createUser(username: string, password: string): User | null {
  if (users.has(username)) {
    console.log(`[AUTH] User already exists: ${username}`);
    return null;
  }

  const passwordHash = hashPassword(password);
  const user: User = {
    id: `user-${Date.now()}`,
    username,
    passwordHash,
    createdAt: new Date()
  };

  users.set(username, user);
  console.log(`[AUTH] User created: ${username}`);
  return user;
}
