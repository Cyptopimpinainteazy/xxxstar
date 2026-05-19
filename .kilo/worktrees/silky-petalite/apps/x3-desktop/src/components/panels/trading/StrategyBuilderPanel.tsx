import React, { useState } from "react";
import { Plus, Trash2, Play, TrendingUp, BarChart3, Zap } from "lucide-react";
import clsx from "clsx";

interface Condition {
  id: string;
  metric: "rsi" | "ma" | "price" | "volume";
  operator: ">" | "<" | "cross";
  value: string;
}

interface Action {
  id: string;
  type: "buy" | "sell";
  amount: string;
  condition?: string;
}

interface Strategy {
  id: string;
  name: string;
  desc: string;
  conditions: Condition[];
  actions: Action[];
  enabled: boolean;
  winRate: number;
  trades: number;
}

const MOCK_STRATEGIES: Strategy[] = [
  {
    id: "1",
    name: "RSI Mean Reversion",
    desc: "Buy when RSI < 30, Sell when RSI > 70",
    conditions: [{ id: "1", metric: "rsi", operator: "<", value: "30" }],
    actions: [{ id: "1", type: "buy", amount: "100" }],
    enabled: true,
    winRate: 62,
    trades: 48,
  },
];

export default function StrategyBuilderPanel() {
  const [strategies, setStrategies] = useState<Strategy[]>(MOCK_STRATEGIES);
  const [selectedId, setSelectedId] = useState<string>(MOCK_STRATEGIES[0].id);
  const [showBuilder, setShowBuilder] = useState(false);
  const [newStrategy, setNewStrategy] = useState({
    name: "New Strategy",
    conditions: [] as Condition[],
    actions: [] as Action[],
  });

  const selected = strategies.find(s => s.id === selectedId);

  const handleAddCondition = () => {
    setNewStrategy({
      ...newStrategy,
      conditions: [...newStrategy.conditions, { id: String(Date.now()), metric: "rsi", operator: ">", value: "50" }],
    });
  };

  const handleAddAction = () => {
    setNewStrategy({
      ...newStrategy,
      actions: [...newStrategy.actions, { id: String(Date.now()), type: "buy", amount: "100" }],
    });
  };

  const handleSaveStrategy = () => {
    const newStrat: Strategy = {
      id: String(Date.now()),
      name: newStrategy.name,
      desc: `${newStrategy.conditions.length} condition${newStrategy.conditions.length !== 1 ? "s" : ""} → ${newStrategy.actions.length} action${newStrategy.actions.length !== 1 ? "s" : ""}`,
      conditions: newStrategy.conditions,
      actions: newStrategy.actions,
      enabled: false,
      winRate: 0,
      trades: 0,
    };
    setStrategies([...strategies, newStrat]);
    setShowBuilder(false);
    setNewStrategy({ name: "New Strategy", conditions: [], actions: [] });
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold">Strategy Builder</h2>
        <button
          onClick={() => setShowBuilder(true)}
          className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded-lg text-sm font-medium transition"
        >
          <Plus size={16} /> New Strategy
        </button>
      </div>

      {/* Strategies List */}
      <div className="flex-1 overflow-y-auto mb-4">
        <div className="space-y-2">
          {strategies.map((strat) => (
            <button
              key={strat.id}
              onClick={() => setSelectedId(strat.id)}
              className={clsx(
                "w-full text-left p-4 rounded-lg border-2 transition",
                selectedId === strat.id
                  ? "border-blue-400 bg-blue-600/10"
                  : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
              )}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <span className="font-semibold">{strat.name}</span>
                  <span className={clsx("w-2 h-2 rounded-full", strat.enabled ? "bg-green-500" : "bg-gray-500")} />
                </div>
                <div className="flex items-center gap-3">
                  <div className="text-right">
                    <div className="text-sm font-semibold text-green-400">{strat.winRate}%</div>
                    <div className="text-xs text-gray-500">win rate</div>
                  </div>
                </div>
              </div>
              <p className="text-xs text-gray-400">{strat.desc}</p>
              <div className="mt-2 text-xs text-gray-500">{strat.trades} trades executed</div>
            </button>
          ))}
        </div>
      </div>

      {/* Strategy Detail */}
      {selected && !showBuilder && (
        <div className="bg-[#15151b] border border-[#2a2a35] p-4 rounded-lg space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="font-bold">{selected.name}</h3>
            <button
              onClick={() => setStrategies(strategies.map(s => s.id === selected.id ? { ...s, enabled: !s.enabled } : s))}
              className={clsx("px-3 py-1 rounded text-sm font-semibold transition", selected.enabled ? "bg-green-600" : "bg-gray-600")}
            >
              {selected.enabled ? "Active" : "Inactive"}
            </button>
          </div>

          <div className="space-y-3">
            <div>
              <h4 className="text-sm font-semibold mb-2 flex items-center gap-2">
                <BarChart3 size={14} /> Conditions
              </h4>
              <div className="space-y-1">
                {selected.conditions.map((cond) => (
                  <div key={cond.id} className="text-xs bg-[#2a2a35] p-2 rounded">
                    {cond.metric.toUpperCase()} {cond.operator} {cond.value}
                  </div>
                ))}
              </div>
            </div>

            <div>
              <h4 className="text-sm font-semibold mb-2 flex items-center gap-2">
                <Zap size={14} /> Actions
              </h4>
              <div className="space-y-1">
                {selected.actions.map((act) => (
                  <div key={act.id} className="text-xs bg-[#2a2a35] p-2 rounded capitalize">
                    {act.type} {act.amount} X3
                  </div>
                ))}
              </div>
            </div>
          </div>

          <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm flex items-center justify-center gap-2 transition">
            <Play size={14} /> Backtest Strategy
          </button>
        </div>
      )}

      {/* Builder Modal */}
      {showBuilder && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-6 w-96 max-h-96 overflow-y-auto">
            <h3 className="text-lg font-bold mb-4">Build Strategy</h3>

            <input
              type="text"
              value={newStrategy.name}
              onChange={(e) => setNewStrategy({ ...newStrategy, name: e.target.value })}
              className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm mb-4"
            />

            <div className="space-y-3 mb-4">
              <div>
                <h4 className="text-sm font-semibold mb-2">Conditions</h4>
                {newStrategy.conditions.map((cond, i) => (
                  <div key={cond.id} className="flex gap-2 mb-2">
                    <select className="flex-1 bg-[#15151b] border border-[#2a2a35] rounded px-2 py-1 text-xs text-white">
                      <option>RSI</option>
                      <option>MA</option>
                      <option>Price</option>
                    </select>
                    <input type="text" placeholder="30" className="w-12 bg-[#15151b] border border-[#2a2a35] rounded px-2 py-1 text-xs text-white" />
                  </div>
                ))}
                <button
                  onClick={handleAddCondition}
                  className="text-xs text-blue-400 hover:text-blue-300 mt-1"
                >
                  + Add Condition
                </button>
              </div>

              <div>
                <h4 className="text-sm font-semibold mb-2">Actions</h4>
                {newStrategy.actions.map((act) => (
                  <div key={act.id} className="flex gap-2 mb-2">
                    <select className="flex-1 bg-[#15151b] border border-[#2a2a35] rounded px-2 py-1 text-xs text-white">
                      <option>Buy</option>
                      <option>Sell</option>
                    </select>
                    <input type="text" placeholder="100" className="w-16 bg-[#15151b] border border-[#2a2a35] rounded px-2 py-1 text-xs text-white" />
                  </div>
                ))}
                <button
                  onClick={handleAddAction}
                  className="text-xs text-blue-400 hover:text-blue-300 mt-1"
                >
                  + Add Action
                </button>
              </div>
            </div>

            <div className="flex gap-2">
              <button
                onClick={() => setShowBuilder(false)}
                className="flex-1 bg-[#15151b] border border-[#2a2a35] py-2 rounded text-sm font-semibold hover:bg-[#1a1a20]"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveStrategy}
                className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded text-sm font-semibold"
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
