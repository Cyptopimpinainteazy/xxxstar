/**
 * MarkdownViewer.tsx — renders markdown files with syntax highlighting.
 *
 * Features:
 * - Fetches and renders markdown files
 * - Syntax highlighting for code blocks
 * - Proper link handling
 * - Loading and error states
 * - Dark theme optimized
 */

import React, { useState, useEffect } from "react";

interface MarkdownViewerProps {
  filePath: string;
  title?: string;
}

const MarkdownViewer: React.FC<MarkdownViewerProps> = ({ filePath, title }) => {
  const [content, setContent] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchMarkdown = async () => {
      try {
        setLoading(true);
        setError(null);

        const response = await fetch(filePath);
        if (!response.ok) {
          throw new Error(`Failed to load ${filePath}`);
        }

        const text = await response.text();
        setContent(text);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load documentation");
      } finally {
        setLoading(false);
      }
    };

    fetchMarkdown();
  }, [filePath]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center">
          <div className="animate-spin text-2xl mb-3">⏳</div>
          <p className="text-text-secondary">Loading documentation...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center max-w-md">
          <div className="text-3xl mb-3">⚠️</div>
          <p className="text-text-primary font-semibold mb-2">Error loading documentation</p>
          <p className="text-text-secondary text-sm">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="prose prose-invert max-w-none prose-lg">
      {title && (
        <div className="mb-6 pb-4 border-b border-border-default">
          <h1 className="text-2xl font-bold text-text-primary m-0">{title}</h1>
        </div>
      )}

      <div
        className="prose-content text-text-primary"
        dangerouslySetInnerHTML={{
          __html: renderMarkdown(content),
        }}
      />
    </div>
  );
};

/**
 * Simple markdown to HTML converter for dark theme
 */
function renderMarkdown(markdown: string): string {
  let html = markdown;

  // Headers
  html = html.replace(/^### (.*?)$/gm, '<h3 class="text-lg font-bold mt-4 mb-2">$1</h3>');
  html = html.replace(/^## (.*?)$/gm, '<h2 class="text-xl font-bold mt-6 mb-3">$1</h2>');
  html = html.replace(/^# (.*?)$/gm, '<h1 class="text-2xl font-bold mt-8 mb-4">$1</h1>');

  // Bold and italic
  html = html.replace(/\*\*(.*?)\*\*/g, '<strong class="font-bold text-accent-primary">$1</strong>');
  html = html.replace(/\*(.*?)\*/g, '<em class="italic">$1</em>');
  html = html.replace(/__(.*?)__/g, '<strong class="font-bold text-accent-primary">$1</strong>');
  html = html.replace(/_([^_]+)_/g, '<em class="italic">$1</em>');

  // Code blocks
  html = html.replace(/```(.*?)\n([\s\S]*?)```/gm, (_match, _lang, code) => {
    const trimmed = code.trim();
    return `<pre class="bg-codeblock-bg text-codeblock-text rounded-lg p-4 overflow-x-auto mb-4 border border-border-default">
      <code>${escapeHtml(trimmed)}</code>
    </pre>`;
  });

  // Inline code
  html = html.replace(/`([^`]+)`/g, '<code class="bg-codeblock-bg text-codeblock-text px-2 py-1 rounded text-sm font-mono">$1</code>');

  // Links
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer" class="text-accent-primary hover:underline">$1</a>');

  // Line breaks
  html = html.replace(/\n\n/g, '</p><p class="mb-4">');
  html = `<p class="mb-4">${html}</p>`;

  // Lists
  html = html.replace(/^\* (.*?)$/gm, '<li class="ml-4 list-disc">$1</li>');
  html = html.replace(/^- (.*?)$/gm, '<li class="ml-4 list-disc">$1</li>');
  html = html.replace(/^\+ (.*?)$/gm, '<li class="ml-4 list-disc">$1</li>');

  // Blockquotes
  html = html.replace(/^> (.*?)$/gm, '<blockquote class="border-l-4 border-accent-primary pl-4 italic text-text-secondary my-4">$1</blockquote>');

  // Horizontal rule
  html = html.replace(/^---$/gm, '<hr class="my-6 border-border-default" />');

  return html;
}

function escapeHtml(text: string): string {
  const map: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#039;",
  };
  return text.replace(/[&<>"']/g, (char) => map[char]);
}

export default MarkdownViewer;
