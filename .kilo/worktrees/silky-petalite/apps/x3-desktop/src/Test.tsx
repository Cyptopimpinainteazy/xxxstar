export default function Test() {
  return (
    <div style={{ 
      background: 'red', 
      color: 'white', 
      padding: '50px', 
      fontSize: '32px',
      minHeight: '100vh'
    }}>
      <h1>🚀 X3 Desktop Test Page</h1>
      <p>If you can see this, React is working!</p>
      <p>Background: Red (to be visible)</p>
      <p>Date: {new Date().toISOString()}</p>
    </div>
  );
}
