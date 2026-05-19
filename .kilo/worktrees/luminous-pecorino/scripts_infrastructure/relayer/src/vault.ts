// Minimal Vault KV v2 helper for relayer (Node/TypeScript)
import fetch from 'node-fetch';

const VAULT_ADDR = process.env.VAULT_ADDR;
const VAULT_TOKEN = process.env.VAULT_TOKEN;

export async function getSecret(path: string, key: string): Promise<string | null> {
  if (!VAULT_ADDR || !VAULT_TOKEN) return null;
  const url = `${VAULT_ADDR.replace(/\/$/, '')}/v1/secret/data/${path}`;
  try {
    const resp = await fetch(url, {
      method: 'GET',
      headers: { 'X-Vault-Token': VAULT_TOKEN }
    });
    if (!resp.ok) return null;
    const body = await resp.json();
    return body?.data?.data?.[key] ?? null;
  } catch (e) {
    // swallow; caller should fallback to env
    return null;
  }
}

export default { getSecret };
