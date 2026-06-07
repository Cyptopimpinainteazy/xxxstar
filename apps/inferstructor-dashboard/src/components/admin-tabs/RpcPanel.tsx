import React from 'react';

interface RpcEndpoint {
  id: string;
  name?: string;
  chain?: string;
  url?: string;
  status?: string;
}

interface RpcPanelProps {
  rpcEndpoints: RpcEndpoint[];
}

export const RpcPanel: React.FC<RpcPanelProps> = ({ rpcEndpoints }) => {
  return (
    <div id="rpc-panel" role="tabpanel" aria-labelledby="rpc" className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
      <h2 className="text-xl font-bold text-white mb-4">RPC Endpoints</h2>
      <div className="space-y-4">
        {rpcEndpoints.map((endpoint) => (
          <div key={endpoint.id} className="flex items-center justify-between p-4 bg-[#0a0a0f] rounded-lg border border-[#2a2a35]">
            <div>
              <p className="text-white font-medium">{endpoint.name || endpoint.chain}</p>
              <p className="text-gray-500 text-sm">{endpoint.url || 'No URL'}</p>
            </div>
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                 <div 
                   className={`w-3 h-3 rounded-full ${endpoint.status === 'up' ? 'bg-green-500' : endpoint.status === 'error' ? 'bg-red-500' : 'bg-yellow-500'}`}
                   aria-label={`Status: ${endpoint.status || 'unknown'}`}
                   role="img"
                 />
                 <span className={`text-sm ${endpoint.status === 'up' ? 'text-green-400' : endpoint.status === 'error' ? 'text-red-400' : 'text-yellow-400'}`}>{endpoint.status || 'unknown'}</span>
               </div>
              <button className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors">
                Edit
              </button>
            </div>
          </div>
        ))}
      </div>
      <button className="mt-4 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors">
        Add Endpoint
      </button>
    </div>
  );
};
