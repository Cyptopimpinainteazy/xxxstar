import React from 'react';

interface RBACRole {
  id: string;
  name: string;
  permissions: string[];
}

interface RBACPanelProps {
  rbacRoles: RBACRole[];
}

export const RBACPanel: React.FC<RBACPanelProps> = ({ rbacRoles }) => {
  return (
    <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
      <h2 className="text-xl font-bold text-white mb-4">Role-Based Access Control</h2>
      <div className="space-y-4">
        {rbacRoles.map((role) => (
          <div key={role.id} className="p-4 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg">
            <h3 className="text-white font-medium mb-3">{role.name}</h3>
            <div className="flex flex-wrap gap-2">
              {role.permissions.map((perm) => (
                <span key={perm} className="px-3 py-1 bg-blue-900 text-blue-300 rounded-full text-xs">
                  {perm.replace(/_/g, ' ')}
                </span>
              ))}
            </div>
            <button className="mt-3 text-blue-400 hover:text-blue-300 text-sm">Edit Permissions</button>
          </div>
        ))}
      </div>
    </div>
  );
};
