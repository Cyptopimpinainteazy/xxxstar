// Use global React/ReactDOM so this file can run directly in the browser
// (Babel-standalone + live-server setup loads this script).
const { useState, useEffect } = React;

function App() {
  const [user, setUser] = useState(null);
  const [projects, setProjects] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Check for existing user
    const checkAuth = async () => {
      try {
        const response = await fetch('/api/users/me');
        if (response.ok) {
          const userData = await response.json();
          setUser(userData);
        }
      } catch (err) {
        console.log('User not authenticated');
      } finally {
        setLoading(false);
      }
    };

    // Fetch projects from backend
    const fetchProjects = async () => {
      try {
        const response = await fetch('/api/projects');
        if (response.ok) {
          const projectData = await response.json();
          setProjects(projectData);
        }
      } catch (err) {
        console.error('Error fetching projects:', err);
      }
    };

    checkAuth();
    fetchProjects();
  }, []);

  // Handle user registration
  const handleRegister = async () => {
    try {
      const response = await fetch('/api/users/register', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          username: 'user' + Math.floor(Math.random() * 1000),
          email: 'user' + Math.floor(Math.random() * 1000) + '@x3.com',
          walletAddress: '0x' + Math.random().toString(16).slice(2, 42),
        }),
      });

      if (response.ok) {
        const userData = await response.json();
        setUser(userData);
      } else {
        alert('Registration failed');
      }
    } catch (err) {
      console.error('Registration error:', err);
      alert('Registration failed');
    }
  };

  // Handle reward claiming
  const handleClaimRewards = async (projectId) => {
    try {
      const response = await fetch('/api/rewards/claim', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ userId: user?.id, rewardIds: [projectId] }),
      });

      if (response.ok) {
        await response.json();
        alert('Rewards claimed successfully!');
      } else {
        alert('Claim failed');
      }
    } catch (err) {
      console.error('Claim error:', err);
      alert('Claim failed');
    }
  };

  if (loading) return React.createElement('div', null, 'Loading...');

  return React.createElement(
    'div',
    null,
    React.createElement('h1', null, 'X3 App Store'),
    React.createElement('p', null, 'Discover and earn from automated crypto apps'),
    !user && React.createElement(
      'div',
      null,
      React.createElement('h2', null, 'Join X3 App Store'),
      React.createElement('p', null, 'Earn 50% profit sharing on all ported apps'),
      React.createElement('button', { onClick: handleRegister }, 'Register Now')
    ),
    projects.length > 0 && React.createElement(
      'div',
      null,
      React.createElement('h2', null, 'Available Apps'),
      React.createElement('div', null, projects.map((project) => (
        React.createElement('div', { key: project.id },
          React.createElement('h3', null, project.name),
          React.createElement('p', null, project.description),
          React.createElement('p', null, 'Status: ' + project.status),
          project.status === 'approved' && React.createElement('button', { onClick: () => handleClaimRewards(project.id) }, 'Claim Rewards')
        )
      )))
    ),

    // Render ProfitSharingPanel when available (loaded via index.html)
    (typeof ProfitSharingPanel !== 'undefined') && React.createElement(ProfitSharingPanel)
  );
}

// Mount app on #root (support React 18+ createRoot when available)
const rootEl = document.getElementById('root');
if (rootEl) {
  if (ReactDOM && ReactDOM.createRoot) {
    ReactDOM.createRoot(rootEl).render(React.createElement(App));
  } else {
    ReactDOM.render(React.createElement(App), rootEl);
  }
}
