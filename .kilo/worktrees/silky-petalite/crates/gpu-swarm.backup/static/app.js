(function () {
  const e = React.createElement;
  const root = document.getElementById('root');

  function fetchJson(path, opts) {
    return fetch(path, opts).then(r => r.json());
  }

  function LoginScreen({onLogin, onRegister}){
    const [token, setToken] = React.useState(localStorage.getItem('gpu_token') || '');
    const [msg, setMsg] = React.useState('');

    function tryLogin(){
      fetchJson('/api/login', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({token})})
        .then(resp => {
          if(resp.ok || resp.ok === undefined){
            localStorage.setItem('gpu_token', token);
            onLogin(token);
          } else {
            setMsg('Invalid token');
          }
        }).catch(e => setMsg('Network error'));
    }

    function register(){
      fetchJson('/api/register',{method:'POST'})
        .then(r => {
          if(r.mnemonic){
            setMsg('SUCCESS! Save your 12-word phrase NOW:\n' + r.mnemonic + '\n\nAddress: ' + r.address);
            setTimeout(() => onRegister(r.address), 15000); // Give them 15 seconds to copy before auto-redirecting
          }
        }).catch(e => setMsg('Error registering'));
    }

    return e('div', {className:'login'},
      e('h2', null, 'GPU Swarm - Local Admin'),
      e('p', null, 'Enter your local admin token to control this node.'),
      e('input', {value: token, onChange: (ev) => setToken(ev.target.value), placeholder:'token'}),
      e('div', {className:'row', style: {marginTop: 10}},
        e('button', {onClick: tryLogin}, 'Login'),
        e('button', {onClick: register, style:{marginLeft:10}}, 'Register (create wallet)')
      ),
      e('pre', {className:'muted', style: {whiteSpace: 'pre-wrap', marginTop: 15, color: '#ffb347', fontWeight: 'bold'}}, msg)
    );
  }

  function Dashboard({token, initialState}){
    const [state, setState] = React.useState(initialState || {rewards_history: []});
    const chartRef = React.useRef(null);

    React.useEffect(() => {
      const ctx = document.getElementById('rewardsChart').getContext('2d');
      chartRef.current = new Chart(ctx, {
        type: 'line',
        data: {
          labels: state.rewards_history.map(p => new Date(p.t*1000).toLocaleTimeString()),
          datasets: [{label:'Rewards', data: state.rewards_history.map(p => p.rewards), borderColor:'blue', fill:false}]
        },
        options: {animation:false}
      });

      const interval = setInterval(()=>{
        fetch('/api/state').then(r => r.json()).then(s => {
          setState(s);
          const labels = s.rewards_history.map(p => new Date(p.t*1000).toLocaleTimeString());
          chartRef.current.data.labels = labels;
          chartRef.current.data.datasets[0].data = s.rewards_history.map(p=>p.rewards);
          chartRef.current.update();
        }).catch(console.error);
      }, 2000);

      return ()=> clearInterval(interval);
    }, []);

    function toggleEnabled(){
      const newState = Object.assign({}, state, {enabled: !state.enabled});
      fetch('/api/state', {method:'POST', headers:{'Content-Type':'application/json','Authorization':'Bearer '+token}, body: JSON.stringify(newState)})
        .then(r=>r.json()).then(s=> setState(s)).catch(console.error);
    }

    function setLevel(level){
      const newState = Object.assign({}, state, {gpu_level: level});
      fetch('/api/state', {method:'POST', headers:{'Content-Type':'application/json','Authorization':'Bearer '+token}, body: JSON.stringify(newState)})
        .then(r=>r.json()).then(s=> setState(s)).catch(console.error);
    }

    function generateNewWallet(){
      if(confirm('Warning: This will overwrite your current node wallet. Continue?')) {
        fetch('/api/register', {method:'POST'}).then(r=>r.json()).then(r => {
           if(r.mnemonic) {
             prompt('SUCCESS! Copy your 12-word recovery phrase NOW (Ctrl+C):', r.mnemonic);
             fetch('/api/state').then(r=>r.json()).then(s=> setState(s));
           }
        });
      }
    }

    return e('div', {className:'dashboard'},
      e('h2', null, 'Node Dashboard'),
      e('div', {style: {display: 'flex', alignItems: 'center', gap: '10px'}}, 
         e('span', null, 'Wallet: ' + (state.wallet_address || 'not registered')),
         e('button', {onClick: generateNewWallet, style: {padding: '4px 8px', fontSize: '0.8rem'}}, 'Force Generate New Wallet')
      ),
      e('div', {style:{marginTop:10}}, 'Enabled: ' + (state.enabled ? 'Yes' : 'No') + ' ' , e('button',{onClick: toggleEnabled}, state.enabled ? 'Turn Off' : 'Turn On')),
      e('div', {style:{marginTop:10}}, 'GPU level: ', ['low','medium','high'].map(l => e('button', {key:l, onClick: ()=>setLevel(l), style:{marginLeft:6}}, l))),
      e('div',{style:{height:300, marginTop:20}}, e('canvas',{id:'rewardsChart'})),
      e('div',{style:{marginTop:10}}, e('p',null,'Uptime: ' + (state.uptime_seconds || 0) + 's'), e('p', null, 'Rewards: ' + (state.rewards || 0).toFixed(4)))
    );
  }

  function App(){
    const [loggedIn, setLoggedIn] = React.useState(!!localStorage.getItem('gpu_token'));
    const [token, setToken] = React.useState(localStorage.getItem('gpu_token') || '');
    const [initialState, setInitialState] = React.useState(null);

    React.useEffect(()=>{
      if(loggedIn){
        fetch('/api/state').then(r=>r.json()).then(s=> setInitialState(s)).catch(console.error);
      }
    },[loggedIn]);

    if(!loggedIn){
      return e(LoginScreen, {onLogin: (t)=>{setToken(t); setLoggedIn(true);}, onRegister: (addr)=>{fetch('/api/state').then(r=>r.json()).then(s=>setInitialState(s)); setLoggedIn(true);}});
    }

    return e(Dashboard, {token, initialState});
  }

  ReactDOM.createRoot(root).render(e(App));
})();