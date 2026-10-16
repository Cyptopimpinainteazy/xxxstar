import { Pool } from 'pg'

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  max: 20,
  idleTimeoutMillis: 30000,
  connectionTimeoutMillis: 2000,
})

export default pool

// Admin operations
export const adminDB = {
  async getAll() {
    const result = await pool.query('SELECT address, added_at FROM admins ORDER BY added_at DESC')
    return result.rows
  },

  async add(address: string) {
    const result = await pool.query(
      'INSERT INTO admins (address, added_at) VALUES ($1, NOW()) ON CONFLICT (address) DO NOTHING RETURNING *',
      [address]
    )
    return result.rows[0]
  },

  async remove(address: string) {
    const result = await pool.query('DELETE FROM admins WHERE address = $1 RETURNING *', [address])
    return result.rows[0]
  },

  async exists(address: string) {
    const result = await pool.query('SELECT 1 FROM admins WHERE address = $1', [address])
    return result.rows.length > 0
  }
}

// Consent operations
export const consentDB = {
  async getAll() {
    const result = await pool.query(`
      SELECT id, contributor_id, wallet, kyc, meta, ts
      FROM consents
      ORDER BY ts DESC
    `)
    return result.rows
  },

  async create(contributorId: string, wallet: string, kyc: boolean = false, meta?: any) {
    const result = await pool.query(
      'INSERT INTO consents (contributor_id, wallet, kyc, meta, ts) VALUES ($1, $2, $3, $4, NOW()) RETURNING *',
      [contributorId, wallet, kyc, JSON.stringify(meta)]
    )
    return result.rows[0]
  },

  async getByWallet(wallet: string) {
    const result = await pool.query('SELECT * FROM consents WHERE wallet = $1', [wallet])
    return result.rows[0]
  }
}

// Claim operations
export const claimDB = {
  async getAll() {
    const result = await pool.query(`
      SELECT id, contributor_id, wallet, amount, status, meta, ts
      FROM claims
      ORDER BY ts DESC
    `)
    return result.rows
  },

  async create(contributorId: string, wallet: string, amount: number, meta?: any) {
    const result = await pool.query(
      'INSERT INTO claims (contributor_id, wallet, amount, status, meta, ts) VALUES ($1, $2, $3, $4, $5, NOW()) RETURNING *',
      [contributorId, wallet, amount, 'queued', JSON.stringify(meta)]
    )
    return result.rows[0]
  },

  async updateStatus(id: number, status: string) {
    const result = await pool.query(
      'UPDATE claims SET status = $1 WHERE id = $2 RETURNING *',
      [status, id]
    )
    return result.rows[0]
  }
}

// Pending Action operations
export const pendingActionDB = {
  async getAll() {
    const result = await pool.query(`
      SELECT id, action_id, contributor_id, wallet, amount, approvals, meta, ts
      FROM pending_actions
      ORDER BY ts DESC
    `)
    return result.rows
  },

  async create(actionId: string, contributorId: string, wallet: string, amount: number, meta?: any) {
    const result = await pool.query(
      'INSERT INTO pending_actions (action_id, contributor_id, wallet, amount, approvals, meta, ts) VALUES ($1, $2, $3, $4, $5, $6, NOW()) RETURNING *',
      [actionId, contributorId, wallet, amount, JSON.stringify([]), JSON.stringify(meta)]
    )
    return result.rows[0]
  },

  async addApproval(id: number, approval: any) {
    const result = await pool.query(
      'UPDATE pending_actions SET approvals = approvals || $1::jsonb WHERE id = $2 RETURNING *',
      [JSON.stringify([approval]), id]
    )
    return result.rows[0]
  },

  async delete(id: number) {
    const result = await pool.query('DELETE FROM pending_actions WHERE id = $1 RETURNING *', [id])
    return result.rows[0]
  }
}

// KYC operations
export const kycDB = {
  async getAll() {
    const result = await pool.query(`
      SELECT id, contributor_id, wallet, provider, ref, verified, verifier, ts
      FROM kyc_entries
      ORDER BY ts DESC
    `)
    return result.rows
  },

  async create(contributorId: string, wallet: string, provider?: string, ref?: string) {
    const result = await pool.query(
      'INSERT INTO kyc_entries (contributor_id, wallet, provider, ref, verified, ts) VALUES ($1, $2, $3, $4, $5, NOW()) RETURNING *',
      [contributorId, wallet, provider, ref, false]
    )
    return result.rows[0]
  },

  async verify(id: number, verifier: string) {
    const result = await pool.query(
      'UPDATE kyc_entries SET verified = true, verifier = $1 WHERE id = $2 RETURNING *',
      [verifier, id]
    )
    return result.rows[0]
  }
}

// Allocation operations
export const allocationDB = {
  async getAll() {
    const result = await pool.query('SELECT id, contributor_id, amount, ts FROM allocations ORDER BY ts DESC')
    return result.rows
  },

  async getByContributor(contributorId: string) {
    const result = await pool.query('SELECT amount FROM allocations WHERE contributor_id = $1', [contributorId])
    return result.rows[0]?.amount || 0
  }
}

// Event operations
export const eventDB = {
  async getAll() {
    const result = await pool.query('SELECT id, type, payload, ts FROM events ORDER BY ts DESC')
    return result.rows
  },

  async create(type: string, payload: any) {
    const result = await pool.query(
      'INSERT INTO events (type, payload, ts) VALUES ($1, $2, NOW()) RETURNING *',
      [type, JSON.stringify(payload)]
    )
    return result.rows[0]
  }
}