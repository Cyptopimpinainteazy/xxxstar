import { useState } from 'react';

export default function TerminalChatBot() {
  const [question, setQuestion] = useState('');
  const [answer, setAnswer] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  async function askBot() {
    if (!question.trim()) return;
    setLoading(true);
    setError('');
    setAnswer('');
    try {
      const res = await fetch('http://localhost:5143/ask', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question })
      });
      const data = await res.json();
      if (data.answer) setAnswer(data.answer);
      else setError(data.error || 'No answer');
    } catch (e) {
      setError('Failed to reach AI backend');
    }
    setLoading(false);
  }

  return (
    <div className="mt-4 border-t border-[#333] pt-3">
      <div className="font-mono text-xs text-[#2ab4cc] mb-1">Ask the X3 AI about the platform, X3 Lang, or docs:</div>
      <div className="flex gap-2">
        <input
          className="flex-1 bg-[#181818] border border-[#444] rounded px-2 py-1 text-sm text-[#e0e0e0] font-mono"
          placeholder="Ask a question..."
          value={question}
          onChange={e => setQuestion(e.target.value)}
          onKeyDown={e => { if (e.key === 'Enter') askBot(); }}
          disabled={loading}
        />
        <button
          className="bg-[#2ab4cc] text-white px-3 py-1 rounded text-xs font-bold font-mono hover:bg-[#1a9fb5]"
          onClick={askBot}
          disabled={loading}
        >Ask</button>
      </div>
      {loading && <div className="text-xs text-[#aaa] mt-2 font-mono">Thinking...</div>}
      {answer && <div className="mt-2 p-2 bg-[#232323] rounded text-[#e0e0e0] text-sm font-mono whitespace-pre-line">{answer}</div>}
      {error && <div className="mt-2 text-xs text-red-400 font-mono">{error}</div>}
    </div>
  );
}
