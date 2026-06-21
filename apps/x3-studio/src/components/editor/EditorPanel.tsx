import { useCallback, useRef, useEffect, useState } from 'react';
import Editor, { OnMount, BeforeMount } from '@monaco-editor/react';
import { useEditorStore, useWorkspaceStore } from '../../store';

const languageMap: Record<string, string> = {
  ts: 'typescript', tsx: 'typescript', js: 'javascript', jsx: 'javascript',
  rs: 'rust', sol: 'sol', py: 'python', json: 'json',
  yaml: 'yaml', yml: 'yaml', toml: 'toml', md: 'markdown',
  html: 'html', css: 'css', x3: 'x3-lang',
};

export default function EditorPanel() {
  const tabs = useEditorStore(s => s.tabs);
  const activeTabId = useEditorStore(s => s.activeTabId);
  const openFile = useEditorStore(s => s.openFile);
  const closeTab = useEditorStore(s => s.closeTab);
  const setActiveTab = useEditorStore(s => s.setActiveTab);
  const updateContent = useEditorStore(s => s.updateContent);
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const editorRef = useRef<any>(null);
  const [monacoReady, setMonacoReady] = useState(false);

  const activeTab = tabs.find(t => t.id === activeTabId);

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
  };

  const handleBeforeMount: BeforeMount = (monaco) => {
    monaco.languages.register({ id: 'x3-lang' });
    monaco.languages.setMonarchTokensProvider('x3-lang', {
      tokenizer: {
        root: [
          [/\/\/.*$/, 'comment'],
          [/\/\*/, 'comment', '@comment'],
          [/".*?"/, 'string'],
          [/'[^']*'/, 'string'],
          [/\b\d+(\.\d+)?\b/, 'number'],
          [/\b(intent|chain|vm|route|lock|claim|refund|finality|oracle|proof|solver|adapter|relayer|quorum|timeout|slashing|scoreboard|require|emit|asset|address|amount|deadline|validator|bridge|swap|proof_ledger|htlc|secret_hash|preimage|settlement)\b/, 'keyword'],
          [/\b(if|else|for|while|return|let|mut|fn|struct|enum|match|import|from|as|true|false)\b/, 'keyword'],
          [/\b(u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|f32|f64|bool|string|Address|Amount|Asset|Hash|Signature|BlockNumber)\b/, 'type'],
          [/[a-zA-Z_$][\w$]*/, 'identifier'],
        ],
        comment: [
          [/[^/*]+/, 'comment'],
          [/\*\//, 'comment', '@pop'],
          [/[/*]/, 'comment'],
        ],
      },
    });

    monaco.languages.registerCompletionItemProvider('x3-lang', {
      provideCompletionItems: (model, position) => {
        const word = model.getWordUntilPosition(position);
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };
        const suggestions = [
          ...['intent', 'chain', 'vm', 'route'].map(k => ({ label: k, kind: monaco.languages.CompletionItemKind.Keyword, insertText: k, range })),
          ...['asset', 'address', 'amount', 'deadline'].map(k => ({ label: k, kind: monaco.languages.CompletionItemKind.Keyword, insertText: k, range })),
          ...['lock', 'claim', 'refund', 'emit'].map(k => ({ label: k, kind: monaco.languages.CompletionItemKind.Function, insertText: `${k} `, range })),
        ];
        return { suggestions };
      },
    });
    setMonacoReady(true);
  };

  const handleFileChange = useCallback(async (filePath: string) => {
    try {
      const content = await window.x3studio.fs.readFile(filePath);
      const ext = filePath.split('.').pop() || '';
      const language = languageMap[ext] || ext;
      openFile(filePath, content, language);
    } catch (err) {
      console.error('Failed to read file:', err);
    }
  }, [openFile]);

  const handleEditorChange = useCallback((value: string | undefined) => {
    if (activeTabId && value !== undefined) {
      updateContent(activeTabId, value);
    }
  }, [activeTabId, updateContent]);

  const handleSave = useCallback(async () => {
    if (!activeTab) return;
    try {
      await window.x3studio.fs.writeFile(activeTab.filePath, activeTab.content);
      useEditorStore.getState().markClean(activeTab.id);
    } catch (err) {
      console.error('Failed to save file:', err);
    }
  }, [activeTab]);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      handleSave();
    }
  }, [handleSave]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  if (!workspacePath) {
    return (
      <div className="editor-area">
        <div className="welcome-message">
          <h2>X3 Studio</h2>
          <p>Open a workspace folder to start developing on X3</p>
          <p style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)' }}>
            Use the sidebar to open a folder or Ctrl+O
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="editor-area">
      <div className="editor-tabs">
        {tabs.map(tab => (
          <div
            key={tab.id}
            className={`editor-tab ${tab.id === activeTabId ? 'active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.dirty && <span className="dirty-dot" />}
            {tab.fileName}
            <span className="close-btn" onClick={(e) => { e.stopPropagation(); closeTab(tab.id); }}>×</span>
          </div>
        ))}
        {tabs.length === 0 && (
          <div style={{ padding: '0 12px', color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)' }}>
            No files open — use File Explorer to open files
          </div>
        )}
      </div>
      <div className="editor-container">
        {activeTab ? (
          <Editor
            key={activeTab.id}
            theme="vs-dark"
            language={activeTab.language === 'x3-lang' ? 'x3-lang' : activeTab.language}
            value={activeTab.content}
            onChange={handleEditorChange}
            beforeMount={handleBeforeMount}
            onMount={handleEditorDidMount}
            options={{
              fontSize: 13,
              fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
              minimap: { enabled: true },
              scrollBeyondLastLine: false,
              lineNumbers: 'on',
              renderWhitespace: 'selection',
              bracketPairColorization: { enabled: true },
              autoClosingBrackets: 'always',
              autoClosingQuotes: 'always',
              folding: true,
              foldingHighlight: true,
              matchBrackets: 'always',
              suggestOnTriggerCharacters: true,
              quickSuggestions: true,
              tabSize: 2,
              wordWrap: 'off',
              smoothScrolling: true,
              cursorBlinking: 'smooth',
              cursorSmoothCaretAnimation: 'on',
              padding: { top: 8 },
            }}
          />
        ) : (
          <div className="welcome-message">
            <p>Open a file from the explorer to start editing</p>
          </div>
        )}
      </div>
    </div>
  );
}
