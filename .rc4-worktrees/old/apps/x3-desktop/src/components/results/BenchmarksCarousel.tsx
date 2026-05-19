import { useState } from "react";

const BENCHMARK_SLIDES = [

  {
    title: "Real-World Gas Savings",
    content: (
      <div className="text-center">
        <p className="mb-2">X3 batch relays 115,000 Ethereum signatures/sec vs. 1,000/sec on CPU.<br />
        <span className="font-semibold">99% less wall time</span>, lower risk of reorgs, more txs per block, less wasted gas.</p>
        <div className="bg-green-50 border border-green-200 rounded p-2 text-green-700 text-xs">“X3 let us batch 10x more txs per block and cut relay costs by 80%.” — Enterprise Validator</div>
      </div>
    ),
  },
  {
    title: "What We Test & Why",
    content: (
      <ul className="list-disc list-inside text-left mx-auto max-w-md text-sm">
        <li><b>Latency:</b> How fast is your RPC? (p50, p90, p99)</li>
        <li><b>Throughput:</b> How many txs/sec can you push?</li>
        <li><b>Reorgs:</b> Can you detect and recover from forks?</li>
        <li><b>Edge Cases:</b> Does it handle bad input safely?</li>
        <li><b>Validator Health:</b> Stake, uptime, liveness</li>
        <li><b>GPU Benchmark:</b> Real ops/sec for 5 crypto kernels</li>
        <li><b>Pool Performance:</b> Mining/staking payout accuracy</li>
      </ul>
    ),
  },
  {
    title: "Customer Quote",
    content: (
      <div className="italic text-center text-lg text-blue-700">“X3 Chain is the only platform that let us run 40+ chains on one GPU and see real-time results. Our revenue per block doubled.”<br /><span className="text-sm text-gray-500">— Multi-Chain Operator</span></div>
    ),
  },
];

export default function BenchmarksCarousel() {
  const [idx, setIdx] = useState(0);
  const next = () => setIdx((i) => (i + 1) % BENCHMARK_SLIDES.length);
  const prev = () => setIdx((i) => (i - 1 + BENCHMARK_SLIDES.length) % BENCHMARK_SLIDES.length);
  return (
    <div className="w-full max-w-xl mx-auto bg-white rounded-lg shadow-lg p-4 relative">
      <div className="mb-2 text-xl font-bold text-center">{BENCHMARK_SLIDES[idx].title}</div>
      <div className="min-h-[120px] flex items-center justify-center">{BENCHMARK_SLIDES[idx].content}</div>
      <div className="flex justify-between mt-4">
        <button onClick={prev} className="px-3 py-1 bg-gray-200 rounded hover:bg-gray-300">Prev</button>
        <button onClick={next} className="px-3 py-1 bg-gray-200 rounded hover:bg-gray-300">Next</button>
      </div>
      <div className="absolute top-2 right-4">
        <a href="/sales" className="text-blue-600 underline text-sm hover:text-blue-800">Click here for more info</a>
      </div>
    </div>
  );
}
