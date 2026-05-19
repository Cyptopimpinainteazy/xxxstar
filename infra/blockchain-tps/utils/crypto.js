const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

const KEY_PATH = process.env.BTPS_KEY_PATH || path.join(require('os').homedir(), '.btps_key');

function ensureKey() {
  if (process.env.BTPS_SECRET && process.env.BTPS_SECRET.length >= 32) {
    return Buffer.from(process.env.BTPS_SECRET, 'utf8').slice(0, 32);
  }
  if (fs.existsSync(KEY_PATH)) {
    return fs.readFileSync(KEY_PATH);
  }
  const k = crypto.randomBytes(32);
  fs.writeFileSync(KEY_PATH, k, { mode: 0o600 });
  return k;
}

const KEY = ensureKey();

function encryptJson(obj) {
  const iv = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv('aes-256-gcm', KEY, iv);
  const plaintext = Buffer.from(JSON.stringify(obj), 'utf8');
  const encrypted = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const tag = cipher.getAuthTag();
  return Buffer.concat([iv, tag, encrypted]).toString('base64');
}

function decryptJson(b64) {
  const data = Buffer.from(b64, 'base64');
  const iv = data.slice(0, 12);
  const tag = data.slice(12, 28);
  const encrypted = data.slice(28);
  const decipher = crypto.createDecipheriv('aes-256-gcm', KEY, iv);
  decipher.setAuthTag(tag);
  const plain = Buffer.concat([decipher.update(encrypted), decipher.final()]);
  return JSON.parse(plain.toString('utf8'));
}

module.exports = { encryptJson, decryptJson, KEY_PATH };
