import { useState } from 'react';
import {
  Menu,
  X,
  Home,
  Users,
  GitMerge,
  CheckCircle,
  Zap,
  DollarSign,
  Settings,
  LogOut,
  BarChart3,
  Shield,
  BookOpen,
  ChevronRight,
} from 'lucide-react';
import type { NavBreadcrumb } from '../App';

interface MainNavProps {
  currentPage: string;
  onNavigate: (page: string, breadcrumb?: NavBreadcrumb[]) => void;
  onLogout: () => void;
  onAdminClick?: () => void;
  breadcrumbs?: NavBreadcrumb[];
  isAdmin?: boolean;
}

export const MainNav: React.FC<MainNavProps> = ({
  currentPage,
  onNavigate,
  onLogout,
  onAdminClick,
  breadcrumbs = [],
  isAdmin = false,
}) => {
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const operatorMenuItems = [
    { id: 'overview', label: 'Overview', icon: Home },
    { id: 'validators', label: 'Validators', icon: Users },
    { id: 'swaps', label: 'Atomic Swaps', icon: GitMerge },
    { id: 'proofs', label: 'Proofs', icon: CheckCircle },
    { id: 'faucet', label: 'Faucet', icon: Zap },
    { id: 'funding', label: 'Funding', icon: DollarSign },
    { id: 'settings', label: 'Settings', icon: Settings },
  ];

  const adminMenuItems = [
    { id: 'admin', label: 'Admin Dashboard', icon: Shield },
    { id: 'validators-admin', label: 'Validator Controls', icon: Users },
    { id: 'leaderboard', label: 'Leaderboard', icon: BarChart3 },
    { id: 'metrics', label: 'Real-time Metrics', icon: BarChart3 },
    { id: 'audit-logs', label: 'Audit Logs', icon: BookOpen },
  ];

  const menuItems = isAdmin ? adminMenuItems : operatorMenuItems;

  const isActive = (id: string) => currentPage === id;

  return (
    <>
      {/* Top Navigation Bar */}
      <div className="fixed top-0 right-0 left-0 bg-[#0f0f1e] border-b border-[#2a2a35] z-40 h-16 flex items-center px-6">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            {breadcrumbs.length > 0 && (
              <>
                {breadcrumbs.map((crumb, idx) => (
                  <div key={idx} className="flex items-center gap-2">
                    <ChevronRight className="w-4 h-4 text-gray-500" />
                    <button
                      onClick={() => onNavigate(crumb.path)}
                      className="text-gray-400 hover:text-white transition-colors text-sm"
                    >
                      {crumb.label}
                    </button>
                  </div>
                ))}
              </>
            )}
          </div>
        </div>

        <div className="flex items-center gap-4">
          {!isAdmin && onAdminClick && (
            <button
              onClick={onAdminClick}
              className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg text-sm transition-colors"
            >
              Admin Access
            </button>
          )}
          {isAdmin && (
            <button
              onClick={() => onNavigate('overview')}
              className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg text-sm transition-colors"
            >
              Exit Admin
            </button>
          )}
          <button
            onClick={onLogout}
            className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm transition-colors flex items-center gap-2"
          >
            <LogOut className="w-4 h-4" />
            Logout
          </button>
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="p-2 hover:bg-gray-800 rounded-lg text-gray-400 hover:text-white transition-colors"
          >
            {sidebarOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
          </button>
        </div>
      </div>

      {/* Sidebar */}
      <div
        className={`fixed left-0 top-0 h-screen bg-[#0a0a0f] border-r border-[#2a2a35] z-30 transition-all duration-300 pt-20 ${
          sidebarOpen ? 'w-64' : 'w-0 overflow-hidden'
        }`}
      >
        <nav className="p-4">
          <div className="space-y-1">
            {menuItems.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.id}
                  onClick={() => onNavigate(item.id)}
                  className={`w-full flex items-center gap-3 px-4 py-2 rounded-lg transition-colors text-sm font-medium ${
                    isActive(item.id)
                      ? 'bg-blue-600 text-white'
                      : 'text-gray-400 hover:bg-gray-900 hover:text-white'
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </div>
        </nav>
      </div>

      {/* Main Content Area */}
      <main className={`transition-all duration-300 pt-16 ${sidebarOpen ? 'ml-64' : 'ml-0'}`}>
        {/* Content is rendered by parent component */}
      </main>
    </>
  );
};
