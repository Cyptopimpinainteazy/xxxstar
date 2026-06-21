import {
  Blocks, FileCode, Terminal, Activity, Database, Search,
  FolderTree, FileJson, Wrench, Globe, Send, Rocket,
  Eye, Code2, type LucideIcon
} from 'lucide-react';

export type PanelId = 'explorer' | 'editor' | 'terminal' | 'network' | 'accounts' | 'contracts'
  | 'files' | 'templates' | 'abis' | 'compiler' | 'rpc' | 'txbuilder' | 'deploy' | 'inspector' | 'events';

interface NavItem {
  id: PanelId;
  icon: LucideIcon;
  label: string;
}

const items: NavItem[] = [
  { id: 'explorer', icon: Blocks, label: 'Chain Explorer' },
  { id: 'files', icon: FolderTree, label: 'Files' },
  { id: 'editor', icon: FileCode, label: 'Editor' },
  { id: 'templates', icon: FileJson, label: 'Templates' },
  { id: 'abis', icon: Search, label: 'ABIs' },
  { id: 'compiler', icon: Code2, label: 'Compiler' },
  { id: 'accounts', icon: Database, label: 'Accounts' },
  { id: 'contracts', icon: Globe, label: 'Contracts' },
  { id: 'txbuilder', icon: Send, label: 'Tx Builder' },
  { id: 'deploy', icon: Rocket, label: 'Deploy' },
  { id: 'inspector', icon: Eye, label: 'Inspector' },
  { id: 'rpc', icon: Terminal, label: 'RPC Console' },
  { id: 'events', icon: Activity, label: 'Events' },
  { id: 'network', icon: Activity, label: 'Network' },
  { id: 'terminal', icon: Terminal, label: 'Terminal' },
];

interface SidebarProps {
  active: PanelId;
  onSelect: (id: PanelId) => void;
}

export function Sidebar({ active, onSelect }: SidebarProps) {
  return (
    <nav style={{
      width: 48, background: '#1e1e1e', borderRight: '1px solid #333',
      display: 'flex', flexDirection: 'column', alignItems: 'center', paddingTop: 8, gap: 2,
      overflowY: 'auto',
    }}>
      {items.map(item => {
        const Icon = item.icon;
        const isActive = active === item.id;
        return (
          <button
            key={item.id}
            title={item.label}
            onClick={() => onSelect(item.id)}
            style={{
              width: 36, height: 36, display: 'flex', alignItems: 'center',
              justifyContent: 'center', border: 'none', borderRadius: 6,
              background: isActive ? '#37373d' : 'transparent',
              color: isActive ? '#fff' : '#888', cursor: 'pointer', fontSize: 18, flexShrink: 0,
            }}
            onMouseEnter={e => { if (!isActive) e.currentTarget.style.background = '#2a2a2a' }}
            onMouseLeave={e => { if (!isActive) e.currentTarget.style.background = 'transparent' }}
          >
            <Icon size={20} />
          </button>
        );
      })}
    </nav>
  );
}
