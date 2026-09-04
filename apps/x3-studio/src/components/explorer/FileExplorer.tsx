import { useState, useEffect, useCallback } from 'react';
import { ChevronRight, ChevronDown, File, Folder, FolderOpen } from 'lucide-react';
import { useEditorStore, useWorkspaceStore } from '../../store';

interface TreeNode {
  name: string;
  path: string;
  isDirectory: boolean;
  children: TreeNode[];
  expanded: boolean;
}

export default function FileExplorer() {
  const [tree, setTree] = useState<TreeNode[]>([]);
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const openFile = useEditorStore(s => s.openFile);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; path: string; isDir: boolean } | null>(null);

  const loadDir = useCallback(async (dirPath: string): Promise<TreeNode[]> => {
    try {
      const entries = await window.x3studio.fs.readDir(dirPath);
      const items: TreeNode[] = [];
      for (const entry of entries) {
        if (entry.name.startsWith('.') || entry.name === 'node_modules' || entry.name === 'target' || entry.name === 'dist') continue;
        items.push({
          name: entry.name,
          path: entry.path,
          isDirectory: entry.isDirectory,
          children: [],
          expanded: false,
        });
      }
      items.sort((a, b) => {
        if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
        return a.name.localeCompare(b.name);
      });
      return items;
    } catch {
      return [];
    }
  }, []);

  useEffect(() => {
    if (workspacePath) {
      loadDir(workspacePath).then(setTree);
    } else {
      setTree([]);
    }
  }, [workspacePath, loadDir]);

  const toggleExpand = async (node: TreeNode) => {
    if (!node.isDirectory) return;
    if (node.children.length === 0) {
      node.children = await loadDir(node.path);
    }
    node.expanded = !node.expanded;
    setTree([...tree]);
  };

  const handleFileClick = async (node: TreeNode) => {
    if (node.isDirectory) {
      await toggleExpand(node);
    } else {
      try {
        const content = await window.x3studio.fs.readFile(node.path);
        const ext = node.name.split('.').pop() || '';
        const languageMap: Record<string, string> = {
          ts: 'typescript', tsx: 'typescript', js: 'javascript', jsx: 'javascript',
          rs: 'rust', sol: 'sol', py: 'python', json: 'json',
          yaml: 'yaml', yml: 'yaml', toml: 'toml', md: 'markdown',
          html: 'html', css: 'css', x3: 'x3-lang',
        };
        openFile(node.path, content, languageMap[ext] || ext);
      } catch (err) {
        console.error('Failed to read file:', err);
      }
    }
  };

  const handleContextMenu = (e: React.MouseEvent, node: TreeNode) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, path: node.path, isDir: node.isDirectory });
  };

  const handleDelete = async () => {
    if (!contextMenu) return;
    try {
      if (contextMenu.isDir) {
        // Would need recursive delete for directories
      } else {
        await window.x3studio.fs.deleteFile(contextMenu.path);
      }
      setContextMenu(null);
      if (workspacePath) {
        const newTree = await loadDir(workspacePath);
        setTree(newTree);
      }
    } catch {}
  };

  useEffect(() => {
    const handler = () => setContextMenu(null);
    window.addEventListener('click', handler);
    return () => window.removeEventListener('click', handler);
  }, []);

  const renderNode = (node: TreeNode, depth: number = 0) => (
    <div key={node.path}>
      <div
        className="tree-node"
        style={{ paddingLeft: 8 + depth * 16 }}
        onClick={() => handleFileClick(node)}
        onContextMenu={(e) => handleContextMenu(e, node)}
      >
        {node.isDirectory ? (
          <>
            {node.expanded ? <ChevronDown className="chevron" /> : <ChevronRight className="chevron" />}
            {node.expanded ? <FolderOpen className="icon" color="var(--yellow)" /> : <Folder className="icon" color="var(--yellow)" />}
          </>
        ) : (
          <>
            <span style={{ width: 12 }} />
            <File className="icon" color="var(--text-muted)" />
          </>
        )}
        <span className="name">{node.name}</span>
      </div>
      {node.isDirectory && node.expanded && node.children.map(child => renderNode(child, depth + 1))}
    </div>
  );

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>File Explorer</span>
        <span style={{ cursor: 'pointer', fontSize: 16 }} onClick={async () => {
          if (workspacePath) setTree(await loadDir(workspacePath));
        }}>↻</span>
      </div>
      <div className="panel-body">
        {tree.length === 0 && (
          <div style={{ color: 'var(--text-muted)', padding: 16, fontSize: 'var(--font-size-sm)' }}>
            No workspace open. Click X3 logo to open a folder.
          </div>
        )}
        <div className="file-tree">
          {tree.map(node => renderNode(node))}
        </div>
      </div>
      {contextMenu && (
        <div
          style={{
            position: 'fixed', left: contextMenu.x, top: contextMenu.y, zIndex: 1000,
            background: 'var(--bg-surface)', border: '1px solid var(--border)',
            borderRadius: 'var(--radius)', padding: '4px 0', minWidth: 120,
          }}
        >
          <div className="tree-node" onClick={handleDelete} style={{ padding: '4px 12px' }}>
            Delete {contextMenu.isDir ? 'Directory' : 'File'}
          </div>
        </div>
      )}
    </div>
  );
}
