import { NextRequest, NextResponse } from 'next/server';
import path from 'node:path';

import { findRepoRoot } from '@/lib/ai/repoRoot';
import { searchRepository } from '@/lib/ai/repoSearch';
import { callOpenRouter } from '@/lib/ai/openrouter';

export const runtime = 'nodejs';

type AskBody = {
  question?: string;
  pagePath?: string;
};

const MAX_QUESTION_CHARS = 1200;
const MAX_CONTEXT_SNIPPETS = 10;

// Very small in-memory rate limiter (dev-friendly). For production, swap to Redis.
const rateWindowMs = 30_000;
const maxRequestsPerWindow = 8;
const ipHits = new Map<string, { ts: number; count: number }>();

function getClientIp(req: NextRequest) {
  const xff = req.headers.get('x-forwarded-for');
  if (xff) return xff.split(',')[0].trim();
  return req.headers.get('x-real-ip') || 'unknown';
}

function rateLimit(ip: string) {
  const now = Date.now();
  const prev = ipHits.get(ip);
  if (!prev || now - prev.ts > rateWindowMs) {
    ipHits.set(ip, { ts: now, count: 1 });
    return { ok: true };
  }
  if (prev.count >= maxRequestsPerWindow) return { ok: false };
  prev.count++;
  return { ok: true };
}

export async function POST(req: NextRequest) {
  const ip = getClientIp(req);
  const rl = rateLimit(ip);
  if (!rl.ok) {
    return NextResponse.json(
      { error: 'Rate limit exceeded. Please wait a moment and try again.' },
      { status: 429 }
    );
  }

  const apiKey = process.env.OPENROUTER_API_KEY;
  if (!apiKey) {
    return NextResponse.json(
      { error: 'OPENROUTER_API_KEY is not configured on the server.' },
      { status: 500 }
    );
  }

  let body: AskBody;
  try {
    body = (await req.json()) as AskBody;
  } catch {
    return NextResponse.json({ error: 'Invalid JSON body' }, { status: 400 });
  }

  const questionRaw = (body.question || '').trim();
  if (!questionRaw) {
    return NextResponse.json({ error: 'Question is required' }, { status: 400 });
  }

  if (questionRaw.length > MAX_QUESTION_CHARS) {
    return NextResponse.json(
      { error: `Question too long (max ${MAX_QUESTION_CHARS} chars)` },
      { status: 400 }
    );
  }

  const pagePath = (body.pagePath || '').slice(0, 200);

  const repoRoot = await findRepoRoot(process.cwd());

  const { snippets, tokens } = await searchRepository({
    repoRoot,
    question: questionRaw,
    maxSnippets: MAX_CONTEXT_SNIPPETS,
  });

  const contextBlocks = snippets.map(s => {
    const rel = s.path.replace(/\\/g, '/');
    const range = `${rel}#L${s.startLine}-L${s.endLine}`;
    return `FILE: ${rel}\nLINES: ${s.startLine}-${s.endLine}\nLINK: ${range}\n---\n${s.text}`;
  });

  const system = [
    'You are Atlas Sphere\'s technical assistant. You answer questions about this repository and platform.',
    'Use ONLY the provided repository context when making claims about this codebase. If context is insufficient, say what file/path to inspect next.',
    'Be concise, actionable, and precise. Prefer bullet points.',
    'If asked to execute OS commands or access the user\'s real terminal, refuse and explain the limitation (browser sandbox).',
    'When referencing code, include the provided LINK lines when possible.',
  ].join('\n');

  const user = [
    pagePath ? `Current page: ${pagePath}` : '',
    `Question: ${questionRaw}`,
    tokens.length ? `Search tokens: ${tokens.join(', ')}` : '',
    '',
    'Repository context (snippets):',
    contextBlocks.length ? contextBlocks.join('\n\n') : '(no relevant snippets found)',
  ].filter(Boolean).join('\n');

  const model = process.env.OPENROUTER_MODEL;

  try {
    const result = await callOpenRouter({
      apiKey,
      model,
      messages: [
        { role: 'system', content: system },
        { role: 'user', content: user },
      ],
    });

    return NextResponse.json({
      answer: result.content,
      model: result.model,
      sources: snippets.map(s => ({
        path: s.path,
        startLine: s.startLine,
        endLine: s.endLine,
      })),
      repoRootHint: path.basename(repoRoot),
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error';
    return NextResponse.json({ error: message }, { status: 500 });
  }
}

export async function HEAD() {
  return new NextResponse(null, { status: 200 });
}
