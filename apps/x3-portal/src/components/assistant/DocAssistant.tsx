/**
 * AI-Powered Documentation Assistant Widget
 * 
 * A conversational AI assistant for Atlas Sphere documentation.
 * Features semantic search, code generation, and multi-turn dialogue.
 * 
 * Built for 2030-era UX with holographic styling and voice support.
 */

'use client';

import React, { useState, useRef, useEffect, useCallback } from 'react';

// =============================================================================
// Types
// =============================================================================

export interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: Date;
  codeBlocks?: CodeBlock[];
  references?: DocReference[];
  isStreaming?: boolean;
}

export interface CodeBlock {
  language: string;
  code: string;
  filename?: string;
  runnable?: boolean;
}

export interface DocReference {
  title: string;
  section: string;
  url: string;
  relevance: number;
}

export interface DocAssistantProps {
  /** Initial system prompt */
  systemPrompt?: string;
  /** API endpoint for AI completion */
  apiEndpoint?: string;
  /** Enable voice input */
  enableVoice?: boolean;
  /** Position on screen */
  position?: 'bottom-right' | 'bottom-left' | 'top-right' | 'top-left';
  /** Theme variant */
  variant?: 'holographic' | 'glass' | 'solid';
  /** Initial collapsed state */
  defaultCollapsed?: boolean;
  /** Suggested questions */
  suggestions?: string[];
}

// =============================================================================
// Constants
// =============================================================================

const DEFAULT_SUGGESTIONS = [
  "How do I deploy a Comit transaction?",
  "Explain the dual-VM architecture",
  "Show me a cross-chain swap example",
  "What's the difference between EVM and SVM?",
  "How do I connect to the testnet RPC?",
  "Generate a Solidity contract template",
];

const SYSTEM_PROMPT = `You are Atlas AI, the documentation assistant for Atlas Sphere - a dual-VM Layer-1 blockchain with EVM and SVM support.

You help developers:
- Understand the Atlas Sphere architecture
- Write smart contracts for EVM (Solidity) and SVM (Rust/Anchor)
- Create Comit transactions that execute atomically across both VMs
- Connect to the testnet and deploy applications
- Debug common issues

Always provide code examples when relevant. Reference official documentation.
Be concise but thorough. Explain complex concepts with analogies.`;

// =============================================================================
// Component
// =============================================================================

export const DocAssistant: React.FC<DocAssistantProps> = ({
  systemPrompt = SYSTEM_PROMPT,
  apiEndpoint = '/api/ai/chat',
  enableVoice = true,
  position = 'bottom-right',
  variant = 'holographic',
  defaultCollapsed = true,
  suggestions = DEFAULT_SUGGESTIONS,
}) => {
  const [isOpen, setIsOpen] = useState(!defaultCollapsed);
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputValue, setInputValue] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [showSuggestions, setShowSuggestions] = useState(true);
  
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const recognitionRef = useRef<any>(null);

  // Position styles
  const positionStyles: Record<string, React.CSSProperties> = {
    'bottom-right': { bottom: 24, right: 24 },
    'bottom-left': { bottom: 24, left: 24 },
    'top-right': { top: 24, right: 24 },
    'top-left': { top: 24, left: 24 },
  };

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Initialize speech recognition
  useEffect(() => {
    if (enableVoice && typeof window !== 'undefined') {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const SpeechRecognitionAPI = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
      if (SpeechRecognitionAPI) {
        recognitionRef.current = new SpeechRecognitionAPI();
        recognitionRef.current.continuous = false;
        recognitionRef.current.interimResults = true;
        recognitionRef.current.lang = 'en-US';

        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        recognitionRef.current.onresult = (event: any) => {
          const transcript = Array.from(event.results)
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            .map((result: any) => result[0].transcript)
            .join('');
          setInputValue(transcript);
        };

        recognitionRef.current.onend = () => {
          setIsListening(false);
        };
      }
    }
  }, [enableVoice]);

  // Generate unique ID
  const generateId = () => `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

  // Send message to AI
  const sendMessage = useCallback(async (content: string) => {
    if (!content.trim() || isLoading) return;

    setShowSuggestions(false);
    
    const userMessage: Message = {
      id: generateId(),
      role: 'user',
      content: content.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInputValue('');
    setIsLoading(true);

    // Create assistant message placeholder
    const assistantId = generateId();
    setMessages((prev) => [
      ...prev,
      {
        id: assistantId,
        role: 'assistant',
        content: '',
        timestamp: new Date(),
        isStreaming: true,
      },
    ]);

    try {
      // Simulated streaming response (in production, use SSE or WebSocket)
      const response = await simulateAIResponse(content, messages);
      
      // Stream the response character by character for effect
      let currentContent = '';
      for (const char of response.content) {
        currentContent += char;
        setMessages((prev) =>
          prev.map((msg) =>
            msg.id === assistantId
              ? { ...msg, content: currentContent }
              : msg
          )
        );
        await new Promise((resolve) => setTimeout(resolve, 10));
      }

      // Finalize message
      setMessages((prev) =>
        prev.map((msg) =>
          msg.id === assistantId
            ? {
                ...msg,
                content: response.content,
                codeBlocks: response.codeBlocks,
                references: response.references,
                isStreaming: false,
              }
            : msg
        )
      );
    } catch (error) {
      console.error('AI response error:', error);
      setMessages((prev) =>
        prev.map((msg) =>
          msg.id === assistantId
            ? {
                ...msg,
                content: 'I apologize, but I encountered an error. Please try again.',
                isStreaming: false,
              }
            : msg
        )
      );
    } finally {
      setIsLoading(false);
    }
  }, [messages, isLoading]);

  // Toggle voice input
  const toggleVoiceInput = () => {
    if (!recognitionRef.current) return;

    if (isListening) {
      recognitionRef.current.stop();
    } else {
      recognitionRef.current.start();
      setIsListening(true);
    }
  };

  // Handle key press
  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage(inputValue);
    }
  };

  // Copy code to clipboard
  const copyCode = async (code: string) => {
    await navigator.clipboard.writeText(code);
  };

  // Run code in sandbox (placeholder)
  const runCode = (code: string, language: string) => {
    // Emit event for CodeSandbox component to handle
    window.dispatchEvent(
      new CustomEvent('run-code-sandbox', { detail: { code, language } })
    );
  };

  return (
    <>
      {/* Floating Button */}
      {!isOpen && (
        <button
          onClick={() => setIsOpen(true)}
          className="doc-assistant-fab"
          style={{
            ...positionStyles[position],
            position: 'fixed',
            zIndex: 9999,
            width: 64,
            height: 64,
            borderRadius: '50%',
            border: 'none',
            background: variant === 'holographic'
              ? 'linear-gradient(135deg, rgba(0, 255, 255, 0.8), rgba(255, 0, 255, 0.8))'
              : 'linear-gradient(135deg, #00d4ff, #0057ff)',
            boxShadow: variant === 'holographic'
              ? '0 0 30px rgba(0, 255, 255, 0.5), inset 0 0 20px rgba(255, 255, 255, 0.2)'
              : '0 8px 32px rgba(0, 100, 255, 0.3)',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.3s ease',
          }}
          aria-label="Open AI Documentation Assistant"
        >
          <svg width="32" height="32" viewBox="0 0 24 24" fill="white">
            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
          </svg>
        </button>
      )}

      {/* Chat Window */}
      {isOpen && (
        <div
          className={`doc-assistant doc-assistant--${variant}`}
          style={{
            ...positionStyles[position],
            position: 'fixed',
            zIndex: 9999,
            width: 420,
            height: 600,
            borderRadius: 16,
            display: 'flex',
            flexDirection: 'column',
            overflow: 'hidden',
            background: variant === 'holographic'
              ? 'linear-gradient(135deg, rgba(10, 15, 30, 0.95), rgba(20, 10, 40, 0.95))'
              : variant === 'glass'
              ? 'rgba(255, 255, 255, 0.1)'
              : '#1a1a2e',
            backdropFilter: 'blur(20px)',
            border: variant === 'holographic'
              ? '1px solid rgba(0, 255, 255, 0.3)'
              : '1px solid rgba(255, 255, 255, 0.1)',
            boxShadow: variant === 'holographic'
              ? '0 0 50px rgba(0, 255, 255, 0.2), 0 0 100px rgba(255, 0, 255, 0.1)'
              : '0 25px 50px rgba(0, 0, 0, 0.25)',
          }}
        >
          {/* Header */}
          <header
            style={{
              padding: '16px 20px',
              borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              {/* Animated AI Avatar */}
              <div
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: '50%',
                  background: 'linear-gradient(135deg, #00ffff, #ff00ff)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  animation: 'pulse 2s ease-in-out infinite',
                }}
              >
                <svg width="24" height="24" viewBox="0 0 24 24" fill="white">
                  <path d="M12 2l2.4 7.4h7.6l-6 4.6 2.3 7-6.3-4.6-6.3 4.6 2.3-7-6-4.6h7.6z"/>
                </svg>
              </div>
              <div>
                <h3 style={{ margin: 0, color: '#fff', fontSize: 16, fontWeight: 600 }}>
                  Atlas AI
                </h3>
                <p style={{ margin: 0, color: 'rgba(255, 255, 255, 0.6)', fontSize: 12 }}>
                  Documentation Assistant
                </p>
              </div>
            </div>
            <button
              onClick={() => setIsOpen(false)}
              style={{
                background: 'none',
                border: 'none',
                color: 'rgba(255, 255, 255, 0.6)',
                cursor: 'pointer',
                padding: 8,
              }}
              aria-label="Close assistant"
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
              </svg>
            </button>
          </header>

          {/* Messages */}
          <div
            style={{
              flex: 1,
              overflowY: 'auto',
              padding: 16,
              display: 'flex',
              flexDirection: 'column',
              gap: 16,
            }}
          >
            {/* Welcome message */}
            {messages.length === 0 && (
              <div style={{ textAlign: 'center', padding: '32px 16px' }}>
                <div
                  style={{
                    width: 80,
                    height: 80,
                    margin: '0 auto 16px',
                    borderRadius: '50%',
                    background: 'linear-gradient(135deg, rgba(0, 255, 255, 0.2), rgba(255, 0, 255, 0.2))',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                  }}
                >
                  <svg width="40" height="40" viewBox="0 0 24 24" fill="rgba(0, 255, 255, 0.8)">
                    <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/>
                  </svg>
                </div>
                <h4 style={{ color: '#fff', margin: '0 0 8px' }}>
                  Welcome to Atlas AI
                </h4>
                <p style={{ color: 'rgba(255, 255, 255, 0.6)', fontSize: 14, margin: 0 }}>
                  I can help you navigate Atlas Sphere documentation, write smart contracts, and debug your code.
                </p>
              </div>
            )}

            {/* Message list */}
            {messages.map((message) => (
              <div
                key={message.id}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: message.role === 'user' ? 'flex-end' : 'flex-start',
                }}
              >
                <div
                  style={{
                    maxWidth: '85%',
                    padding: '12px 16px',
                    borderRadius: message.role === 'user' ? '16px 16px 4px 16px' : '16px 16px 16px 4px',
                    background: message.role === 'user'
                      ? 'linear-gradient(135deg, #0066ff, #00aaff)'
                      : 'rgba(255, 255, 255, 0.1)',
                    color: '#fff',
                    fontSize: 14,
                    lineHeight: 1.5,
                  }}
                >
                  <div style={{ whiteSpace: 'pre-wrap' }}>{message.content}</div>

                  {/* Code blocks */}
                  {message.codeBlocks?.map((block, i) => (
                    <div
                      key={i}
                      style={{
                        marginTop: 12,
                        background: 'rgba(0, 0, 0, 0.3)',
                        borderRadius: 8,
                        overflow: 'hidden',
                      }}
                    >
                      <div
                        style={{
                          padding: '8px 12px',
                          background: 'rgba(0, 0, 0, 0.2)',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                        }}
                      >
                        <span style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.6)' }}>
                          {block.filename || block.language}
                        </span>
                        <div style={{ display: 'flex', gap: 8 }}>
                          <button
                            onClick={() => copyCode(block.code)}
                            style={{
                              background: 'none',
                              border: 'none',
                              color: 'rgba(255, 255, 255, 0.6)',
                              cursor: 'pointer',
                              fontSize: 12,
                            }}
                          >
                            Copy
                          </button>
                          {block.runnable && (
                            <button
                              onClick={() => runCode(block.code, block.language)}
                              style={{
                                background: 'rgba(0, 255, 100, 0.2)',
                                border: 'none',
                                color: '#00ff64',
                                cursor: 'pointer',
                                fontSize: 12,
                                padding: '2px 8px',
                                borderRadius: 4,
                              }}
                            >
                              Run
                            </button>
                          )}
                        </div>
                      </div>
                      <pre
                        style={{
                          margin: 0,
                          padding: 12,
                          fontSize: 13,
                          overflowX: 'auto',
                          fontFamily: 'JetBrains Mono, monospace',
                        }}
                      >
                        <code>{block.code}</code>
                      </pre>
                    </div>
                  ))}

                  {/* References */}
                  {message.references && message.references.length > 0 && (
                    <div style={{ marginTop: 12 }}>
                      <p style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.6)', marginBottom: 8 }}>
                        📚 Related Documentation:
                      </p>
                      {message.references.map((ref, i) => (
                        <a
                          key={i}
                          href={ref.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          style={{
                            display: 'block',
                            padding: '8px 12px',
                            background: 'rgba(0, 100, 255, 0.1)',
                            borderRadius: 8,
                            marginBottom: 4,
                            color: '#00aaff',
                            textDecoration: 'none',
                            fontSize: 13,
                          }}
                        >
                          {ref.title}
                          <span style={{ color: 'rgba(255, 255, 255, 0.4)', marginLeft: 8 }}>
                            {ref.section}
                          </span>
                        </a>
                      ))}
                    </div>
                  )}

                  {/* Streaming indicator */}
                  {message.isStreaming && (
                    <span
                      style={{
                        display: 'inline-block',
                        width: 8,
                        height: 16,
                        background: '#00ffff',
                        animation: 'blink 1s infinite',
                        marginLeft: 2,
                      }}
                    />
                  )}
                </div>
                <span
                  style={{
                    fontSize: 11,
                    color: 'rgba(255, 255, 255, 0.4)',
                    marginTop: 4,
                  }}
                >
                  {message.timestamp.toLocaleTimeString()}
                </span>
              </div>
            ))}

            <div ref={messagesEndRef} />
          </div>

          {/* Suggestions */}
          {showSuggestions && messages.length === 0 && (
            <div
              style={{
                padding: '0 16px 16px',
                display: 'flex',
                flexWrap: 'wrap',
                gap: 8,
              }}
            >
              {suggestions.slice(0, 4).map((suggestion, i) => (
                <button
                  key={i}
                  onClick={() => sendMessage(suggestion)}
                  style={{
                    background: 'rgba(0, 255, 255, 0.1)',
                    border: '1px solid rgba(0, 255, 255, 0.3)',
                    borderRadius: 20,
                    padding: '8px 16px',
                    color: '#00ffff',
                    cursor: 'pointer',
                    fontSize: 12,
                    transition: 'all 0.2s ease',
                  }}
                >
                  {suggestion}
                </button>
              ))}
            </div>
          )}

          {/* Input */}
          <div
            style={{
              padding: 16,
              borderTop: '1px solid rgba(255, 255, 255, 0.1)',
            }}
          >
            <div
              style={{
                display: 'flex',
                gap: 8,
                background: 'rgba(255, 255, 255, 0.05)',
                borderRadius: 12,
                padding: '8px 12px',
              }}
            >
              <textarea
                ref={inputRef}
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                onKeyPress={handleKeyPress}
                placeholder="Ask about Atlas Sphere..."
                rows={1}
                style={{
                  flex: 1,
                  background: 'none',
                  border: 'none',
                  color: '#fff',
                  resize: 'none',
                  fontSize: 14,
                  lineHeight: 1.5,
                  outline: 'none',
                }}
              />
              {enableVoice && (
                <button
                  onClick={toggleVoiceInput}
                  style={{
                    background: isListening ? 'rgba(255, 0, 0, 0.3)' : 'none',
                    border: 'none',
                    color: isListening ? '#ff0066' : 'rgba(255, 255, 255, 0.6)',
                    cursor: 'pointer',
                    padding: 8,
                    borderRadius: 8,
                  }}
                  aria-label={isListening ? 'Stop listening' : 'Start voice input'}
                >
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zm-1-9c0-.55.45-1 1-1s1 .45 1 1v6c0 .55-.45 1-1 1s-1-.45-1-1V5zm6 6c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"/>
                  </svg>
                </button>
              )}
              <button
                onClick={() => sendMessage(inputValue)}
                disabled={!inputValue.trim() || isLoading}
                style={{
                  background: inputValue.trim()
                    ? 'linear-gradient(135deg, #00ffff, #0066ff)'
                    : 'rgba(255, 255, 255, 0.1)',
                  border: 'none',
                  borderRadius: 8,
                  padding: '8px 16px',
                  color: '#fff',
                  cursor: inputValue.trim() ? 'pointer' : 'not-allowed',
                  fontWeight: 600,
                  fontSize: 14,
                }}
              >
                {isLoading ? '...' : 'Send'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* CSS Animation Keyframes */}
      <style>{`
        @keyframes pulse {
          0%, 100% { transform: scale(1); box-shadow: 0 0 20px rgba(0, 255, 255, 0.5); }
          50% { transform: scale(1.05); box-shadow: 0 0 30px rgba(255, 0, 255, 0.5); }
        }
        @keyframes blink {
          0%, 50% { opacity: 1; }
          51%, 100% { opacity: 0; }
        }
      `}</style>
    </>
  );
};

// =============================================================================
// Simulated AI Response (Replace with real API in production)
// =============================================================================

interface AIResponse {
  content: string;
  codeBlocks?: CodeBlock[];
  references?: DocReference[];
}

async function simulateAIResponse(
  query: string,
  history: Message[]
): Promise<AIResponse> {
  // Simulate network latency
  await new Promise((resolve) => setTimeout(resolve, 500));

  const queryLower = query.toLowerCase();

  // Pattern matching for demo responses
  if (queryLower.includes('comit') || queryLower.includes('transaction')) {
    return {
      content: `A **Comit transaction** is Atlas Sphere's atomic cross-VM operation. It bundles EVM and SVM calls into a single, all-or-nothing execution unit.

Here's how to submit one:`,
      codeBlocks: [
        {
          language: 'javascript',
          filename: 'submit-comit.js',
          runnable: true,
          code: `import { AtlasSDK } from '@atlas-sphere/sdk';

const atlas = new AtlasSDK({ rpcUrl: 'http://localhost:9944' });

const comit = await atlas.createComit({
  evmPayload: {
    contract: '0x...',
    method: 'swap',
    args: [tokenA, amount],
  },
  svmPayload: {
    program: 'SwapProgramId',
    instruction: 'complete_swap',
    accounts: [...],
  },
});

const receipt = await comit.submit();
console.log('Comit hash:', receipt.comitHash);`,
        },
      ],
      references: [
        {
          title: 'Comit Transactions',
          section: 'Core Concepts',
          url: '/docs/concepts/comit',
          relevance: 0.95,
        },
        {
          title: 'Cross-VM Tutorial',
          section: 'Tutorials',
          url: '/docs/tutorials/cross-vm',
          relevance: 0.88,
        },
      ],
    };
  }

  if (queryLower.includes('evm') || queryLower.includes('solidity')) {
    return {
      content: `The **EVM layer** in Atlas Sphere is Ethereum-compatible. You can deploy standard Solidity contracts and interact with them using familiar tools like ethers.js or web3.js.

Here's a basic token contract:`,
      codeBlocks: [
        {
          language: 'solidity',
          filename: 'AtlasToken.sol',
          code: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract AtlasToken is ERC20 {
    constructor() ERC20("Atlas Token", "ATLAS") {
        _mint(msg.sender, 1000000 * 10 ** decimals());
    }
}`,
        },
      ],
      references: [
        {
          title: 'EVM Integration',
          section: 'Architecture',
          url: '/docs/architecture#evm',
          relevance: 0.92,
        },
      ],
    };
  }

  if (queryLower.includes('svm') || queryLower.includes('solana') || queryLower.includes('rust')) {
    return {
      content: `The **SVM layer** runs Solana-compatible programs using BPF bytecode. You can write programs in Rust using the Anchor framework.

Here's an Anchor program skeleton:`,
      codeBlocks: [
        {
          language: 'rust',
          filename: 'lib.rs',
          code: `use anchor_lang::prelude::*;

declare_id!("YourProgramId");

#[program]
pub mod atlas_swap {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Atlas Swap initialized!");
        Ok(())
    }

    pub fn swap(ctx: Context<Swap>, amount: u64) -> Result<()> {
        // Swap logic here
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct Swap {}`,
        },
      ],
      references: [
        {
          title: 'SVM Integration',
          section: 'Architecture',
          url: '/docs/architecture#svm',
          relevance: 0.94,
        },
        {
          title: 'Anchor Tutorial',
          section: 'Tutorials',
          url: '/docs/tutorials/svm-hello',
          relevance: 0.85,
        },
      ],
    };
  }

  if (queryLower.includes('testnet') || queryLower.includes('rpc') || queryLower.includes('connect')) {
    return {
      content: `To connect to Atlas Sphere Testnet:

**RPC Endpoint:** \`http://rpc.testnet.atlas-sphere.io:9944\`
**Chain ID:** \`1337\`
**Explorer:** \`https://explorer.testnet.atlas-sphere.io\`
**Faucet:** \`https://faucet.testnet.atlas-sphere.io\`

Here's how to connect:`,
      codeBlocks: [
        {
          language: 'javascript',
          filename: 'connect.js',
          runnable: true,
          code: `import { ethers } from 'ethers';

const provider = new ethers.JsonRpcProvider(
  'http://rpc.testnet.atlas-sphere.io:9944'
);

const blockNumber = await provider.getBlockNumber();
console.log('Current block:', blockNumber);`,
        },
      ],
      references: [
        {
          title: 'Testnet Guide',
          section: 'Getting Started',
          url: '/docs/quickstart#testnet',
          relevance: 0.97,
        },
      ],
    };
  }

  // Default response
  return {
    content: `I can help you with Atlas Sphere! Here are some things I can assist with:

• **Smart Contracts** – Write and deploy EVM (Solidity) or SVM (Rust/Anchor) contracts
• **Comit Transactions** – Create atomic cross-VM operations
• **SDK Usage** – Examples in JavaScript, Rust, and CLI
• **Debugging** – Troubleshoot common issues

What would you like to know more about?`,
    references: [
      {
        title: 'Getting Started',
        section: 'Overview',
        url: '/docs/getting-started',
        relevance: 0.8,
      },
    ],
  };
}

export default DocAssistant;
