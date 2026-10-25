"""Opt-in volunteer lease verifier. Not connected to legacy shell-command worker.

Uses HMAC-SHA256 for coordinator authentication and SQLite for durable
single-use nonce protection. A trusted deployment must provision a unique
high-entropy shared key to each volunteer through an authenticated channel.
"""
import hashlib
import hmac
import json
import sqlite3
import time

FIELDS = ("worker_id", "job_id", "nonce", "expires", "manifest_sha256")

def canonical(lease):
    if not isinstance(lease, dict) or set(lease) != set(FIELDS):
        raise ValueError("invalid lease fields")
    if any(not isinstance(lease[k], str) or not lease[k] for k in
           ("worker_id", "job_id", "nonce", "manifest_sha256")):
        raise ValueError("invalid lease identifiers")
    if type(lease["expires"]) is not int or lease["expires"] <= 0:
        raise ValueError("invalid expiry")
    digest = lease["manifest_sha256"]
    if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        raise ValueError("invalid manifest digest")
    if len(lease["nonce"]) > 128 or len(lease["job_id"]) > 128:
        raise ValueError("identifier too long")
    return json.dumps(lease, sort_keys=True, separators=(",", ":")).encode()

def sign(lease, secret):
    if len(secret) < 32:
        raise ValueError("secret too short")
    return hmac.new(secret, canonical(lease), hashlib.sha256).hexdigest()

class LeaseVerifier:
    def __init__(self, db_path, secret):
        if len(secret) < 32:
            raise ValueError("secret too short")
        self.secret = secret
        self.db = sqlite3.connect(db_path, isolation_level=None)
        self.db.execute("PRAGMA busy_timeout=5000")
        self.db.execute("CREATE TABLE IF NOT EXISTS used_leases (nonce TEXT PRIMARY KEY, expires INTEGER NOT NULL)")

    def accept(self, lease, signature, worker_id, job_id, manifest_bytes, now=None):
        payload = canonical(lease)
        if not isinstance(signature, str) or len(signature) != 64:
            raise ValueError("invalid signature")
        expected = hmac.new(self.secret, payload, hashlib.sha256).hexdigest()
        if not hmac.compare_digest(expected, signature):
            raise ValueError("unauthenticated lease")
        if lease["worker_id"] != worker_id or lease["job_id"] != job_id:
            raise ValueError("wrong worker or job")
        if not hmac.compare_digest(lease["manifest_sha256"],
                                   hashlib.sha256(manifest_bytes).hexdigest()):
            raise ValueError("manifest mismatch")
        now = int(time.time()) if now is None else now
        if now >= lease["expires"]:
            raise ValueError("expired lease")
        try:
            self.db.execute("BEGIN IMMEDIATE")
            self.db.execute("INSERT INTO used_leases (nonce, expires) VALUES (?, ?)",
                            (lease["nonce"], lease["expires"]))
            self.db.execute("COMMIT")
        except sqlite3.Error as exc:
            self.db.execute("ROLLBACK")
            raise ValueError("replayed lease or unavailable replay store") from exc
        return True

    def close(self):
        self.db.close()
