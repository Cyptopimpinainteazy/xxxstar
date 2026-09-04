import { Suspense, lazy, useEffect, useState } from 'react';
import { MainNav } from './components/MainNav';
import { ToastProvider } from './components/Toast';
import { api } from './api';

const RegisterPage = lazy(() => import('./components/RegisterPage').then(module => ({ default: module.RegisterPage })));
const LoginPage = lazy(() => import('./components/LoginPage').then(module => ({ default: module.LoginPage })));
const Dashboard = lazy(() => import('./components/Dashboard').then(module => ({ default: module.Dashboard })));
const AdminLogin = lazy(() => import('./components/AdminLogin').then(module => ({ default: module.AdminLogin })));
const AdminDashboard = lazy(() => import('./components/AdminDashboard').then(module => ({ default: module.AdminDashboard })));
const TpsLeaderboard = lazy(() => import('./components/TpsLeaderboard').then(module => ({ default: module.TpsLeaderboard })));
const ValidatorControls = lazy(() => import('./components/ValidatorControls').then(module => ({ default: module.ValidatorControls })));
const AdminControls = lazy(() => import('./components/AdminControls').then(module => ({ default: module.AdminControls })));
const LeaderboardAndMetrics = lazy(() => import('./components/LeaderboardAndMetrics').then(module => ({ default: module.LeaderboardAndMetrics })));

type AuthPage = 'register' | 'login';
type OperatorPage = 'overview' | 'validators' | 'swaps' | 'proofs' | 'faucet' | 'funding' | 'settings';
type AdminPage = 'admin-login' | 'admin' | 'validators-admin' | 'leaderboard' | 'metrics' | 'audit-logs';
type Page = AuthPage | OperatorPage | AdminPage;

export interface NavBreadcrumb {
  label: string;
  path: OperatorPage | AdminPage;
}

function PageFallback() {
  return (
    <div className="flex min-h-[40vh] items-center justify-center px-6">
      <div className="rounded-lg border border-gray-800 bg-gray-900/70 px-4 py-3 text-sm text-gray-300">
        Loading dashboard module...
      </div>
    </div>
  );
}

function AuthFallback() {
  return (
    <div className="flex min-h-screen items-center justify-center px-6">
      <div className="rounded-lg border border-gray-800 bg-gray-900/70 px-4 py-3 text-sm text-gray-300">
        Loading authentication flow...
      </div>
    </div>
  );
}

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('register');
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isAdmin, setIsAdmin] = useState(false);
  const [breadcrumbs, setBreadcrumbs] = useState<NavBreadcrumb[]>([]);

  useEffect(() => {
    // Check if user is already authenticated
    if (api.isAuthenticated()) {
      setIsAuthenticated(true);
      setCurrentPage('overview');
    }
  }, []);

  const handleRegisterSuccess = () => {
    setIsAuthenticated(true);
    setCurrentPage('overview');
  };

  const handleLoginSuccess = () => {
    setIsAuthenticated(true);
    setCurrentPage('overview');
  };

  const handleLogout = () => {
    api.logout();
    api.adminLogout();
    setIsAuthenticated(false);
    setIsAdmin(false);
    setCurrentPage('register');
    setBreadcrumbs([]);
  };

  const handleAdminClick = async () => {
    // If already has valid admin token, go straight to admin
    const valid = await api.verifyAdminToken();
    if (valid) {
      setIsAdmin(true);
      setCurrentPage('admin');
      setBreadcrumbs([]);
    } else {
      setCurrentPage('admin-login');
    }
  };

  const navigateTo = (page: OperatorPage | AdminPage, breadcrumb?: NavBreadcrumb[]) => {
    setCurrentPage(page as Page);
    setBreadcrumbs(breadcrumb || []);
  };

  const goToLogin = () => {
    setCurrentPage('login');
  };

  const goToRegister = () => {
    setCurrentPage('register');
  };

  return (
    <ToastProvider>
      <div className="min-h-screen">
        {/* Auth Pages */}
        {currentPage === 'register' && (
          <>
            <Suspense fallback={<AuthFallback />}>
              <RegisterPage onRegisterSuccess={handleRegisterSuccess} />
            </Suspense>
            <div className="fixed bottom-6 right-6">
              <button
                onClick={goToLogin}
                className="px-6 py-2 bg-gray-800 hover:bg-gray-700 text-white rounded-lg transition-colors"
              >
                Already have an account? <span className="text-blue-400">Log In</span>
              </button>
            </div>
          </>
        )}
        {currentPage === 'login' && (
          <Suspense fallback={<AuthFallback />}>
            <LoginPage onLoginSuccess={handleLoginSuccess} onBackToRegister={goToRegister} />
          </Suspense>
        )}

        {/* Admin Auth */}
        {currentPage === 'admin-login' && isAuthenticated && (
          <Suspense fallback={<PageFallback />}>
            <AdminLogin
              onLoginSuccess={() => {
                setIsAdmin(true);
                setCurrentPage('admin');
              }}
              onBack={() => setCurrentPage('overview')}
            />
          </Suspense>
        )}

        {/* Operator Pages with Navigation */}
        {isAuthenticated && !isAdmin && ['overview', 'validators', 'swaps', 'proofs', 'faucet', 'funding', 'settings'].includes(currentPage as string) && (
          <>
            <MainNav
              currentPage={currentPage as OperatorPage}
              onNavigate={(page: string) => navigateTo(page as OperatorPage | AdminPage)}
              onLogout={handleLogout}
              onAdminClick={handleAdminClick}
              breadcrumbs={breadcrumbs}
            />
            <div className="ml-64 pt-4 pb-4">
              <Suspense fallback={<PageFallback />}>
                {currentPage === 'overview' && (
                  <Dashboard onLogout={handleLogout} onAdmin={handleAdminClick} onLeaderboard={() => setCurrentPage('leaderboard' as Page)} />
                )}
                {currentPage === 'validators' && (
                  <ValidatorControls />
                )}
              </Suspense>
              {currentPage === 'swaps' && (
                <div className="px-6">
                  <h1 className="text-3xl font-bold text-white mb-6">Atomic Swaps</h1>
                  <p className="text-gray-400">Cross-chain swap tracking and management coming soon</p>
                </div>
              )}
              {currentPage === 'proofs' && (
                <div className="px-6">
                  <h1 className="text-3xl font-bold text-white mb-6">Proofs</h1>
                  <p className="text-gray-400">Proof verification and management coming soon</p>
                </div>
              )}
              {currentPage === 'faucet' && (
                <div className="px-6">
                  <h1 className="text-3xl font-bold text-white mb-6">Faucet</h1>
                  <p className="text-gray-400">Testnet faucet management coming soon</p>
                </div>
              )}
              {currentPage === 'funding' && (
                <div className="px-6">
                  <h1 className="text-3xl font-bold text-white mb-6">Funding</h1>
                  <p className="text-gray-400">Funding controls and account management coming soon</p>
                </div>
              )}
              {currentPage === 'settings' && (
                <div className="px-6">
                  <h1 className="text-3xl font-bold text-white mb-6">Settings</h1>
                  <p className="text-gray-400">Application settings and preferences coming soon</p>
                </div>
              )}
            </div>
          </>
        )}

        {/* Admin Pages */}
        {isAdmin && (currentPage === 'admin' || currentPage === 'leaderboard' || currentPage === 'metrics' || currentPage === 'audit-logs' || currentPage === 'validators-admin') && (
          <>
            <MainNav
              currentPage={currentPage as AdminPage}
              onNavigate={(page: string) => navigateTo(page as OperatorPage | AdminPage)}
              onLogout={handleLogout}
              breadcrumbs={breadcrumbs}
              isAdmin={true}
            />
            <div className="ml-64 pt-4 pb-4">
              <Suspense fallback={<PageFallback />}>
                {currentPage === 'admin' && (
                  <AdminDashboard onBack={() => { setIsAdmin(false); setCurrentPage('overview'); }} />
                )}
                {currentPage === 'leaderboard' && (
                  <TpsLeaderboard onBack={() => setCurrentPage('admin')} />
                )}
                {currentPage === 'metrics' && (
                  <LeaderboardAndMetrics />
                )}
              </Suspense>
              {currentPage === 'audit-logs' && (
                <div className="px-6">
                  <h1 className="text-3xl font-bold text-white mb-6">Audit Logs</h1>
                  <p className="text-gray-400">Audit log viewer coming soon</p>
                </div>
              )}
              <Suspense fallback={<PageFallback />}>
                {currentPage === 'validators-admin' && (
                  <AdminControls />
                )}
              </Suspense>
            </div>
          </>
        )}
      </div>
    </ToastProvider>
  );
}

export default App;
