import React from 'react';

interface AuditLog {
  id: string;
  action: string;
  actor: string;
  timestamp: string;
  status: 'success' | 'failed' | 'pending';
}

interface AuditPanelProps {
  auditLogs: AuditLog[];
}

export const AuditPanel: React.FC<AuditPanelProps> = ({ auditLogs }) => {
  return (
    <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
      <table className="w-full">
        <thead>
          <tr className="border-b border-[#2a2a35]">
            <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Action</th>
            <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Actor</th>
            <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Timestamp</th>
            <th className="px-6 py-4 text-left text-sm font-semibold text-gray-300">Status</th>
          </tr>
        </thead>
        <tbody>
          {auditLogs.map((log) => (
            <tr key={log.id} className="border-b border-[#2a2a35] hover:bg-[#2a2a35] transition-colors">
              <td className="px-6 py-4 text-white">{log.action}</td>
              <td className="px-6 py-4 text-gray-300">{log.actor}</td>
              <td className="px-6 py-4 text-gray-300">{log.timestamp}</td>
              <td className="px-6 py-4">
                <span
                  className={`px-3 py-1 rounded-full text-xs font-medium ${
                    log.status === 'success'
                      ? 'bg-green-900 text-green-300'
                      : log.status === 'failed'
                      ? 'bg-red-900 text-red-300'
                      : 'bg-yellow-900 text-yellow-300'
                  }`}
                >
                  {log.status}
                </span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};
