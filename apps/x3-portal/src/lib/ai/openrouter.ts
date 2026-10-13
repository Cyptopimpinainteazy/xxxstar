type ChatMessage = { role: 'system' | 'user' | 'assistant'; content: string };

export type OpenRouterResponse = {
  id?: string;
  choices?: Array<{ message?: { role?: string; content?: string } }>;
  error?: { message?: string };
};

const FALLBACK_MODELS = [
  // Prefer user-configured model first, then try commonly-available options.
  // Note: availability can change; we handle failures by falling back.
  'openrouter/auto',
];

export async function callOpenRouter(params: {
  apiKey: string;
  model?: string;
  messages: ChatMessage[];
  maxTokens?: number;
  temperature?: number;
}) {
  const {
    apiKey,
    model,
    messages,
    maxTokens = 700,
    temperature = 0.2,
  } = params;

  const modelCandidates = [model, ...FALLBACK_MODELS].filter(Boolean) as string[];

  let lastError: unknown;

  for (const m of modelCandidates) {
    try {
      const res = await fetch('https://openrouter.ai/api/v1/chat/completions', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${apiKey}`,
          'Content-Type': 'application/json',
          // Optional but recommended by OpenRouter
          'X-Title': 'Atlas Sphere Explorer',
        },
        body: JSON.stringify({
          model: m,
          messages,
          max_tokens: maxTokens,
          temperature,
        }),
      });

      const json = (await res.json()) as OpenRouterResponse;
      if (!res.ok) {
        const msg = json?.error?.message || `OpenRouter request failed (${res.status})`;
        lastError = new Error(msg);
        continue;
      }

      const content = json?.choices?.[0]?.message?.content;
      if (!content) {
        lastError = new Error('OpenRouter returned no content');
        continue;
      }

      return { model: m, content };
    } catch (err) {
      lastError = err;
      continue;
    }
  }

  throw lastError instanceof Error ? lastError : new Error('Failed to call OpenRouter');
}
