import React, { useState } from "react";
import { ChevronDown, Plus, Trash2, TrendingUp, TrendingDown, Clock, AlertCircle, CheckCircle } from "lucide-react";
import clsx from "clsx";

interface Order {
  id: string;
  pair: string;
  type: "limit" | "stop-loss" | "twap";
  direction: "buy" | "sell";
  triggerPrice?: number;
  limitPrice?: number;
  totalSize: number;
  filledSize: number;
  status: "pending" | "filled" | "cancelled";
  createdAt: string;
  estimatedFill?: string;
}

const MOCK_ORDERS: Order[] = [
  {
    id: "1",
    pair: "X3/USDC",
    type: "limit",
    direction: "buy",
    limitPrice: 0.95,
    totalSize: 1000,
    filledSize: 650,
    status: "pending",
    createdAt: "2 hours ago",
    estimatedFill: "$617.50",
  },
  {
    id: "2",
    pair: "ETH/USDC",
    type: "stop-loss",
    direction: "sell",
    triggerPrice: 2800,
    limitPrice: 2790,
    totalSize: 5,
    filledSize: 0,
    status: "pending",
    createdAt: "1 hour ago",
  },
  {
    id: "3",
    pair: "USDC/X3",
    type: "twap",
    direction: "sell",
    triggerPrice: 1.02,
    totalSize: 500,
    filledSize: 180,
    status: "pending",
    createdAt: "30 min ago",
  },
];

export default function DexAdvancedOrdersPanel() {
  const [orders, setOrders] = useState<Order[]>(MOCK_ORDERS);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [createData, setCreateData] = useState<{
    pair: string;
    type: 'limit' | 'stop-loss' | 'twap';
    direction: 'buy' | 'sell';
    size: number;
    limitPrice: number;
    triggerPrice: number;
  }>({
    pair: "X3/USDC",
    type: "limit",
    direction: "buy",
    size: 100,
    limitPrice: 0.95,
    triggerPrice: 1.05,
  });

  const handleCreateOrder = () => {
    const newOrder: Order = {
      id: String(orders.length + 1),
      pair: createData.pair,
      type: createData.type,
      direction: createData.direction,
      totalSize: createData.size,
      filledSize: 0,
      status: "pending",
      createdAt: "now",
      limitPrice: createData.limitPrice,
      triggerPrice: createData.triggerPrice,
    };
    setOrders([...orders, newOrder]);
    setShowCreateModal(false);
  };

  const handleCancelOrder = (id: string) => {
    setOrders(orders.map(o => o.id === id ? { ...o, status: "cancelled" as const } : o));
  };

  const getOrderTypeColor = (type: string): string => {
    switch (type) {
      case "limit": return "bg-blue-500/20 text-blue-300";
      case "stop-loss": return "bg-red-500/20 text-red-300";
      case "twap": return "bg-purple-500/20 text-purple-300";
      default: return "bg-gray-500/20 text-gray-300";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold">Advanced Orders</h2>
        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded-lg text-sm font-medium transition"
        >
          <Plus size={16} /> New Order
        </button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Active Orders</div>
          <div className="text-2xl font-bold">{orders.filter(o => o.status === "pending").length}</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Total Committed</div>
          <div className="text-2xl font-bold">${(orders.reduce((sum, o) => sum + (o.totalSize * (o.limitPrice || o.triggerPrice || 1)), 0) / 1000).toFixed(1)}K</div>
        </div>
        <div className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35]">
          <div className="text-xs text-gray-400 mb-1">Fill Ratio</div>
          <div className="text-2xl font-bold">
            {orders.length > 0 ? ((orders.reduce((sum, o) => sum + o.filledSize, 0) / orders.reduce((sum, o) => sum + o.totalSize, 0)) * 100).toFixed(0) : 0}%
          </div>
        </div>
      </div>

      {/* Orders List */}
      <div className="flex-1 overflow-y-auto">
        {orders.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-400">
            <AlertCircle size={48} className="opacity-30" />
          </div>
        ) : (
          <div className="space-y-3">
            {orders.map((order) => (
              <div
                key={order.id}
                className="bg-[#15151b] p-4 rounded-lg border border-[#2a2a35] hover:border-[#3a3a45] transition"
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-start gap-3 flex-1">
                    <div>
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-semibold">{order.pair}</span>
                        <span className={clsx("text-xs font-semibold px-2 py-1 rounded-full", getOrderTypeColor(order.type))}>
                          {order.type.toUpperCase()}
                        </span>
                        <span className={clsx("text-xs font-semibold px-2 py-1 rounded-full", order.direction === "buy" ? "bg-green-500/20 text-green-300" : "bg-red-500/20 text-red-300")}>
                          {order.direction === "buy" ? "BUY" : "SELL"}
                        </span>
                        <span className={clsx("text-xs px-2 py-1 rounded", order.status === "pending" ? "bg-yellow-500/20 text-yellow-300" : order.status === "filled" ? "bg-green-500/20 text-green-300" : "bg-gray-500/20 text-gray-300")}>
                          {order.status}
                        </span>
                      </div>
                      <div className="text-xs text-gray-400">{order.createdAt}</div>
                    </div>
                  </div>
                  {order.status === "pending" && (
                    <button
                      onClick={() => handleCancelOrder(order.id)}
                      className="p-2 hover:bg-red-500/20 rounded text-red-400 transition"
                    >
                      <Trash2 size={16} />
                    </button>
                  )}
                </div>

                {/* Order Details */}
                <div className="grid grid-cols-4 gap-3 text-sm mb-3">
                  <div>
                    <div className="text-xs text-gray-400 mb-0.5">Size</div>
                    <div className="font-semibold">{order.totalSize}</div>
                    <div className="text-xs text-gray-500">Filled: {order.filledSize}</div>
                  </div>
                  {order.limitPrice && (
                    <div>
                      <div className="text-xs text-gray-400 mb-0.5">Limit Price</div>
                      <div className="font-semibold">${order.limitPrice.toFixed(4)}</div>
                    </div>
                  )}
                  {order.triggerPrice && (
                    <div>
                      <div className="text-xs text-gray-400 mb-0.5">Trigger</div>
                      <div className="font-semibold">${order.triggerPrice.toFixed(4)}</div>
                    </div>
                  )}
                  {order.estimatedFill && (
                    <div>
                      <div className="text-xs text-gray-400 mb-0.5">Est. Fill Value</div>
                      <div className="font-semibold">{order.estimatedFill}</div>
                    </div>
                  )}
                </div>

                {/* Progress Bar */}
                <div className="w-full bg-[#2a2a35] rounded-full h-1.5">
                  <div
                    className="bg-blue-500 h-1.5 rounded-full transition-all"
                    style={{ width: `${(order.filledSize / order.totalSize) * 100}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Create Order Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-6 w-96">
            <h3 className="text-lg font-bold mb-4">Create New Order</h3>

            <div className="space-y-4">
              <div>
                <label className="text-sm text-gray-400 block mb-2">Pair</label>
                <select
                  value={createData.pair}
                  onChange={(e) => setCreateData({ ...createData, pair: e.target.value })}
                  className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm"
                >
                  <option>X3/USDC</option>
                  <option>ETH/USDC</option>
                  <option>BTC/USDC</option>
                </select>
              </div>

              <div>
                <label className="text-sm text-gray-400 block mb-2">Order Type</label>
                <div className="flex gap-2">
                  {(["limit", "stop-loss", "twap"] as const).map((type) => (
                    <button
                      key={type}
                      onClick={() => setCreateData({ ...createData, type })}
                      className={clsx(
                        "flex-1 py-2 rounded px-3 text-sm font-semibold transition",
                        createData.type === type ? "bg-blue-600 text-white" : "bg-[#15151b] text-gray-400 border border-[#2a2a35]"
                      )}
                    >
                      {type.toUpperCase()}
                    </button>
                  ))}
                </div>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <div>
                  <label className="text-sm text-gray-400 block mb-2">Direction</label>
                  <div className="flex gap-1">
                    {(["buy", "sell"] as const).map((dir) => (
                      <button
                        key={dir}
                        onClick={() => setCreateData({ ...createData, direction: dir })}
                        className={clsx(
                          "flex-1 py-1 rounded text-xs font-semibold transition",
                          createData.direction === dir
                            ? dir === "buy" ? "bg-green-600 text-white" : "bg-red-600 text-white"
                            : "bg-[#15151b] text-gray-400 border border-[#2a2a35]"
                        )}
                      >
                        {dir.toUpperCase()}
                      </button>
                    ))}
                  </div>
                </div>
                <div>
                  <label className="text-sm text-gray-400 block mb-2">Size</label>
                  <input
                    type="number"
                    value={createData.size}
                    onChange={(e) => setCreateData({ ...createData, size: Number(e.target.value) })}
                    className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-1 text-white text-sm"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <div>
                  <label className="text-sm text-gray-400 block mb-2">Limit Price</label>
                  <input
                    type="number"
                    value={createData.limitPrice}
                    onChange={(e) => setCreateData({ ...createData, limitPrice: Number(e.target.value) })}
                    className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-1 text-white text-sm"
                  />
                </div>
                <div>
                  <label className="text-sm text-gray-400 block mb-2">Trigger Price</label>
                  <input
                    type="number"
                    value={createData.triggerPrice}
                    onChange={(e) => setCreateData({ ...createData, triggerPrice: Number(e.target.value) })}
                    className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-1 text-white text-sm"
                  />
                </div>
              </div>
            </div>

            <div className="flex gap-2 mt-6">
              <button
                onClick={() => setShowCreateModal(false)}
                className="flex-1 bg-[#15151b] border border-[#2a2a35] py-2 rounded text-sm font-semibold hover:bg-[#1a1a20] transition"
              >
                Cancel
              </button>
              <button
                onClick={handleCreateOrder}
                className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded text-sm font-semibold transition"
              >
                Create Order
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
