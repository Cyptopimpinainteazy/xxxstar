import { useEffect, useState, useCallback } from 'react';
import { invoke } from '../../ipc/tauri';

interface SwarmTask {
  id: string;
  title?: string;
  name?: string;
  status?: string;
  agent?: string;
  priority?: number;
  risk?: string;
  feature?: string;
  created_at?: string;
}

interface SwarmActivityResult {
  tasks?: SwarmTask[];
  error?: string;
  swarm_api?: string;
}

function SwarmActivityPanel() {
  const [tasks, setTasks] = useState<SwarmTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchTasks = useCallback(async () => {
    try {
      const result = await invoke<SwarmActivityResult>('get_swarm_activity');
      const taskList = result?.tasks;
      if (Array.isArray(taskList)) {
        setTasks(taskList);
        setError(result.error || null);
      } else if (taskList) {
        setTasks([taskList as unknown as SwarmTask]);
      } else {
        setTasks([]);
      }
    } catch (err) {
      console.error('Failed to fetch swarm activity:', err);
      setError('Swarm API unreachable');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTasks();
    const interval = setInterval(fetchTasks, 5000);
    return () => clearInterval(interval);
  }, [fetchTasks]);

  const pending = tasks.filter(t => t.status === 'Pending' || t.status === 'pending').length;
  const running = tasks.filter(t => t.status === 'Running' || t.status === 'running').length;
  const passed = tasks.filter(t => t.status === 'Passed' || t.status === 'passed').length;
  const failed = tasks.filter(t => t.status === 'Failed' || t.status === 'failed').length;

  if (loading) {
    return (
      <div className="view p-6">
        <h2 className="text-xl font-bold text-white mb-2">Swarm Activity</h2>
        <div className="text-gray-400">Loading swarm task list via Tauri → x3-swarm-api :8787...</div>
      </div>
    );
  }

  return (
    <div className="view p-6">
      <div className="mb-4">
        <h2 className="text-xl font-bold text-white">Swarm Activity</h2>
        <p className="text-gray-400 text-sm">Agent task queue from x3-swarm-api</p>
      </div>

      {error && <div className="bg-yellow-900/30 border border-yellow-600/30 rounded-lg p-3 mb-4 text-yellow-300 text-sm">{error}</div>}

      <div className="grid grid-cols-4 gap-3 mb-6">
        <div className="bg-gray-800/40 rounded-lg p-3 border border-gray-700/50"><div className="text-gray-500 text-xs">Pending</div><div className="text-yellow-400 font-mono text-xl">{pending}</div></div>
        <div className="bg-gray-800/40 rounded-lg p-3 border border-gray-700/50"><div className="text-gray-500 text-xs">Running</div><div className="text-blue-400 font-mono text-xl">{running}</div></div>
        <div className="bg-gray-800/40 rounded-lg p-3 border border-gray-700/50"><div className="text-gray-500 text-xs">Passed</div><div className="text-green-400 font-mono text-xl">{passed}</div></div>
        <div className="bg-gray-800/40 rounded-lg p-3 border border-gray-700/50"><div className="text-gray-500 text-xs">Failed</div><div className="text-red-400 font-mono text-xl">{failed}</div></div>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-gray-400 border-b border-gray-700">
              <th className="py-2 px-3">Task ID</th>
              <th className="py-2 px-3">Title</th>
              <th className="py-2 px-3">Agent</th>
              <th className="py-2 px-3">Status</th>
              <th className="py-2 px-3">Risk</th>
            </tr>
          </thead>
          <tbody>
            {tasks.length > 0 ? tasks.map((task, i) => (
              <tr key={task.id || i} className="border-b border-gray-800 hover:bg-gray-800/30">
                <td className="py-2 px-3 font-mono text-xs text-cyan-400">{task.id || `#${i}`}</td>
                <td className="py-2 px-3 text-white text-sm">{task.title || task.name || 'Untitled'}</td>
                <td className="py-2 px-3 font-mono text-xs text-gray-400">{task.agent || '-'}</td>
                <td className="py-2 px-3">
                  <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                    task.status === 'Passed' || task.status === 'passed' ? 'bg-green-900/40 text-green-400' :
                    task.status === 'Failed' || task.status === 'failed' ? 'bg-red-900/40 text-red-400' :
                    task.status === 'Running' || task.status === 'running' ? 'bg-blue-900/40 text-blue-400' :
                    'bg-gray-800/40 text-gray-400'
                  }`}>
                    {task.status || 'Pending'}
                  </span>
                </td>
                <td className="py-2 px-3 font-mono text-xs">
                  <span className={task.risk === 'high' ? 'text-red-400' : task.risk === 'medium' ? 'text-yellow-400' : 'text-gray-400'}>
                    {task.risk || 'low'}
                  </span>
                </td>
              </tr>
            )) : (
              <tr>
                <td colSpan={5} className="py-8 text-center text-gray-500">
                  No swarm tasks found. Start x3-swarm-api to populate the task queue.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="mt-3 text-xs text-gray-600">
        Query: invoke('get_swarm_activity') → x3-swarm-api :8787/tasks
      </div>
    </div>
  );
}

export default SwarmActivityPanel;