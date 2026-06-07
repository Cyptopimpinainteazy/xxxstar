import { useState, useEffect } from 'react';
import { RegisterPage } from './components/RegisterPage';
import { LoginPage } from './components/LoginPage';
import { Dashboard } from './components/Dashboard';
import { AdminLogin } from './components/AdminLogin';
import { AdminDashboard } from './components/AdminDashboard';
import { ChainExplorer } from './components/ChainExplorer';
import { api } from './api';

type Page = 'register' | 'login' | 'dashboard' | 'admin-login' | 'admin' | 'chains';

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('register');
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  useEffect(() => {
    // Check if user is already authenticated
    if (api.isAuthenticated()) {
      setIsAuthenticated(true);
      setCurrentPage('dashboard');
    }
  }, []);

  const handleRegisterSuccess = () => {
    setIsAuthenticated(true);
    setCurrentPage('dashboard');
  };

  const handleLoginSuccess = () => {
    setIsAuthenticated(true);
    setCurrentPage('dashboard');
  };

  const handleLogout = () => {
    api.logout();
    api.adminLogout();
    setIsAuthenticated(false);
    setCurrentPage('register');
  };

  const handleAdminClick = async () => {
    // If already has valid admin token, go straight to admin
    const valid = await api.verifyAdminToken();
    if (valid) {
      setCurrentPage('admin');
    } else {
      setCurrentPage('admin-login');
    }
  };

  const goToLogin = () => {
    setCurrentPage('login');
  };

  const goToRegister = () => {
    setCurrentPage('register');
  };

  return (
    <div className="min-h-screen">
      {currentPage === 'register' && (
        <>
          <RegisterPage onRegisterSuccess={handleRegisterSuccess} />
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
        <LoginPage onLoginSuccess={handleLoginSuccess} onBackToRegister={goToRegister} />
      )}
      {currentPage === 'dashboard' && isAuthenticated && (
        <Dashboard onLogout={handleLogout} onAdmin={handleAdminClick} onChains={() => setCurrentPage('chains')} />
      )}
      {currentPage === 'admin-login' && isAuthenticated && (
        <AdminLogin
          onLoginSuccess={() => setCurrentPage('admin')}
          onBack={() => setCurrentPage('dashboard')}
        />
      )}
      {currentPage === 'admin' && isAuthenticated && (
        <AdminDashboard onBack={() => setCurrentPage('dashboard')} />
      )}
      {currentPage === 'chains' && isAuthenticated && (
        <ChainExplorer onBack={() => setCurrentPage('dashboard')} />
      )}
    </div>
  );
}

export default App;
