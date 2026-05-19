import React, { useState, useEffect } from "react";
import {
  runSystemCommand,
  listAdminCommands,
  type AllowedCommand,
} from "@/services/adminService";

const SystemCommandPanel: React.FC = () => {
  const [output, setOutput] = useState("");
  const [loading, setLoading] = useState(false);
  const [commands, setCommands] = useState<AllowedCommand[]>([]);

  useEffect(() => {
    listAdminCommands()
      .then(setCommands)
      .catch(() => setCommands([]));
  }, []);

  const runCommand = async (cmdId: string) => {
    setLoading(true);
    setOutput("");
    try {
      const result = await runSystemCommand(cmdId);
      setOutput(`$ ${cmdId}\n\n${result}`);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setOutput(`Error: ${msg}`);
    }
    setLoading(false);
  };

  return (
    <div style={{background:'#181818',color:'#ffd740',padding:24,borderRadius:12,maxWidth:700,margin:'32px auto'}}>
      <h2 style={{marginBottom:16}}>System Command Panel</h2>
      <div style={{display:'flex',flexWrap:'wrap',gap:12,marginBottom:16}}>
        {commands.map((cmd) => (
          <button
            key={cmd.id}
            style={{background:'#222',color:'#ffd740',padding:'8px 18px',borderRadius:6,fontWeight:'bold',border:'1px solid #ffd740'}}
            onClick={() => runCommand(cmd.id)}
            disabled={loading}
          >
            {cmd.id}
          </button>
        ))}
      </div>
      <pre style={{background:'#222',color:'#fff',padding:16,borderRadius:8,minHeight:120,marginTop:8,whiteSpace:'pre-wrap'}}>
        {loading ? 'Running...' : output}
      </pre>
    </div>
  );
};

export default SystemCommandPanel;
