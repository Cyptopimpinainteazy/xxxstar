import{c as k,r as o,j as e,Z as S,P as D,D as _}from"./index-CJcSO9kX.js";import{c as l}from"./clsx-B-dksMZM.js";import{C as z}from"./code-xml-CTZyEJQz.js";import{C as y}from"./circle-check-big-vIynnMI-.js";import{C as n}from"./circle-alert-CiwcWGRm.js";import{C as A}from"./copy-Bn8csAhk.js";/**
 * @license lucide-react v0.563.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const I=[["circle",{cx:"18",cy:"5",r:"3",key:"gq8acd"}],["circle",{cx:"6",cy:"12",r:"3",key:"w7nqdw"}],["circle",{cx:"18",cy:"19",r:"3",key:"1xt0gg"}],["line",{x1:"8.59",x2:"15.42",y1:"13.51",y2:"17.49",key:"47mynk"}],["line",{x1:"15.41",x2:"8.59",y1:"6.51",y2:"10.49",key:"1n3mei"}]],P=k("share-2",I),x=[{id:"1",name:"Simple Token",language:"x3lang",code:`contract SimpleToken {
  storage {
    balances: Map<Address, U256>;
    total_supply: U256;
  }

  public fn mint(amount: U256) {
    balances[tx.origin] += amount;
    total_supply += amount;
  }

  public fn transfer(to: Address, amount: U256) -> bool {
    require(balances[tx.origin] >= amount);
    balances[tx.origin] -= amount;
    balances[to] += amount;
    true
  }
}`},{id:"2",name:"Voting Contract",language:"x3lang",code:`contract Voting {
  storage {
    proposals: Vec<Proposal>;
    votes: Map<(U256, Address), bool>;
  }

  struct Proposal {
    description: String;
    vote_count: U256;
    executed: bool;
  }

  public fn create_proposal(desc: String) {
    proposals.push(Proposal {
      description: desc,
      vote_count: 0,
      executed: false
    });
  }

  public fn vote(proposal_id: U256) {
    require(!votes[(proposal_id, tx.origin)]);
    votes[(proposal_id, tx.origin)] = true;
    proposals[proposal_id].vote_count += 1;
  }
}`}],T={success:!0,message:"Compilation successful",bytecode:"0x608060405234801561001057600080fd5b5061012f806100206000396000f3fe",abi:{contract:"SimpleToken",functions:["mint","transfer"]},gas:45678,timestamp:new Date().toISOString()};function q(){const[m,h]=o.useState(x[0]),[d,b]=o.useState(x[0].code),[t,u]=o.useState(null),[a,p]=o.useState(null),[r,j]=o.useState("editor"),[c,f]=o.useState(!1),[i,g]=o.useState(!1),v=()=>{f(!0),setTimeout(()=>{u(T),f(!1)},1500)},N=()=>{if(!t||!t.success){alert("Please compile successfully first");return}g(!0),setTimeout(()=>{p({success:!0,contractAddress:"0x742d35Cc6634C0532925a3b844Bc9e7595f3bEb0",txHash:"0x8c7d6dfe1c0a6bc20f8a8c9b3c9c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",blockNumber:18527842,gasUsed:2456789,timestamp:new Date().toISOString()}),g(!1)},2e3)},C=()=>{navigator.clipboard.writeText(d)},w=()=>{const s=document.createElement("a");s.setAttribute("href","data:text/plain;charset=utf-8,"+encodeURIComponent(d)),s.setAttribute("download",`${m.name}.x3`),s.style.display="none",document.body.appendChild(s),s.click(),document.body.removeChild(s)};return e.jsxs("div",{className:"w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col",children:[e.jsxs("h2",{className:"text-xl font-bold mb-4 flex items-center gap-2",children:[e.jsx(z,{size:20,className:"text-green-400"})," Interactive Code Playground"]}),e.jsxs("div",{className:"flex-1 overflow-hidden space-y-4 mb-4",children:[e.jsx("div",{className:"flex gap-2 overflow-x-auto",children:x.map(s=>e.jsx("button",{onClick:()=>{h(s),b(s.code),u(null),p(null)},className:l("px-3 py-2 rounded-lg border transition text-sm font-semibold whitespace-nowrap",m.id===s.id?"border-green-600 bg-green-600/10 text-green-400":"border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"),children:s.name},s.id))}),e.jsxs("div",{className:"flex-1 flex gap-3 overflow-hidden",children:[e.jsxs("div",{className:"flex-1 flex flex-col border border-[#2a2a35] rounded-lg overflow-hidden",children:[e.jsx("div",{className:"flex gap-2 bg-[#15151b] border-b border-[#2a2a35] px-3 py-2",children:["editor","output","deployed"].map(s=>e.jsxs("button",{onClick:()=>j(s),className:l("px-3 py-1 text-xs font-semibold rounded transition",r===s?"bg-green-600/20 text-green-400":"text-gray-400 hover:text-gray-300"),children:[s==="editor"&&"Editor",s==="output"&&"Compilation",s==="deployed"&&"Deployment"]},s))}),r==="editor"&&e.jsx("div",{className:"flex-1 flex flex-col overflow-hidden",children:e.jsx("textarea",{value:d,onChange:s=>b(s.target.value),className:"flex-1 bg-[#0a0a0f] text-gray-300 p-3 font-mono text-xs focus:outline-none resize-none",spellCheck:!1})}),r==="output"&&t&&e.jsxs("div",{className:"flex-1 overflow-y-auto p-3 space-y-2 bg-[#0a0a0f]",children:[e.jsxs("div",{className:l("p-2 rounded flex items-center gap-2",t.success?"bg-green-600/10 text-green-400":"bg-red-600/10 text-red-400"),children:[t.success?e.jsx(y,{size:14}):e.jsx(n,{size:14}),e.jsx("span",{className:"text-xs font-semibold",children:t.message})]}),t.gas&&e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs",children:[e.jsx("div",{className:"text-gray-400",children:"Gas Estimate"}),e.jsxs("div",{className:"font-bold text-yellow-400 mt-0.5",children:[t.gas.toLocaleString()," units"]})]}),t.bytecode&&e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded p-2",children:[e.jsx("div",{className:"text-xs text-gray-400 mb-1",children:"Bytecode"}),e.jsx("div",{className:"bg-[#0a0a0f] rounded p-2 text-xs font-mono text-gray-400 break-all max-h-24 overflow-y-auto",children:t.bytecode})]}),t.abi&&e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded p-2",children:[e.jsx("div",{className:"text-xs text-gray-400 mb-1",children:"ABI"}),e.jsx("div",{className:"bg-[#0a0a0f] rounded p-2 text-xs font-mono text-gray-400",children:JSON.stringify(t.abi,null,2)})]})]}),r==="deployed"&&a&&e.jsxs("div",{className:"flex-1 overflow-y-auto p-3 space-y-2 bg-[#0a0a0f]",children:[e.jsxs("div",{className:l("p-2 rounded flex items-center gap-2",a.success?"bg-green-600/10 text-green-400":"bg-red-600/10 text-red-400"),children:[a.success?e.jsx(y,{size:14}):e.jsx(n,{size:14}),e.jsx("span",{className:"text-xs font-semibold",children:a.success?"Deployment successful":a.error})]}),a.contractAddress&&e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs",children:[e.jsx("div",{className:"text-gray-400",children:"Contract Address"}),e.jsx("div",{className:"font-mono text-cyan-400 mt-0.5 break-all",children:a.contractAddress})]}),a.txHash&&e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs",children:[e.jsx("div",{className:"text-gray-400",children:"Transaction Hash"}),e.jsx("div",{className:"font-mono text-cyan-400 mt-0.5 break-all",children:a.txHash})]}),a.blockNumber&&e.jsxs("div",{className:"grid grid-cols-2 gap-2",children:[e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs",children:[e.jsx("div",{className:"text-gray-400",children:"Block Number"}),e.jsx("div",{className:"font-bold text-yellow-400 mt-0.5",children:a.blockNumber})]}),e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs",children:[e.jsx("div",{className:"text-gray-400",children:"Gas Used"}),e.jsxs("div",{className:"font-bold text-yellow-400 mt-0.5",children:[((a.gasUsed??0)/1e6).toFixed(2),"M"]})]})]})]}),!t&&r==="output"&&e.jsx("div",{className:"flex-1 flex items-center justify-center text-gray-500 bg-[#0a0a0f]",children:e.jsxs("div",{className:"text-center",children:[e.jsx(n,{size:32,className:"mx-auto mb-2 opacity-50"}),e.jsx("div",{className:"text-xs",children:"Compile code to see output"})]})}),!a&&r==="deployed"&&e.jsx("div",{className:"flex-1 flex items-center justify-center text-gray-500 bg-[#0a0a0f]",children:e.jsxs("div",{className:"text-center",children:[e.jsx(n,{size:32,className:"mx-auto mb-2 opacity-50"}),e.jsx("div",{className:"text-xs",children:"Deploy to see deployment details"})]})})]}),e.jsxs("div",{className:"flex flex-col gap-2 w-32",children:[e.jsxs("button",{onClick:v,disabled:c,className:l("flex-1 flex flex-col items-center justify-center gap-1 rounded-lg border transition font-semibold text-xs py-2",c?"bg-yellow-600/10 border-yellow-600/50 cursor-not-allowed":"bg-[#15151b] border-[#2a2a35] hover:border-yellow-600"),children:[e.jsx(S,{size:14})," ",c?"...":"Compile"]}),e.jsxs("button",{onClick:N,disabled:i||!t||!t.success,className:l("flex-1 flex flex-col items-center justify-center gap-1 rounded-lg border transition font-semibold text-xs py-2",i||!t?"bg-green-600/10 border-green-600/30 cursor-not-allowed opacity-50":"bg-[#15151b] border-[#2a2a35] hover:border-green-600"),children:[e.jsx(D,{size:14})," ",i?"...":"Deploy"]}),e.jsxs("button",{onClick:C,className:"flex-1 flex flex-col items-center justify-center gap-1 bg-[#15151b] border border-[#2a2a35] hover:border-cyan-600 rounded-lg transition font-semibold text-xs py-2",children:[e.jsx(A,{size:14})," Copy"]}),e.jsxs("button",{onClick:w,className:"flex-1 flex flex-col items-center justify-center gap-1 bg-[#15151b] border border-[#2a2a35] hover:border-cyan-600 rounded-lg transition font-semibold text-xs py-2",children:[e.jsx(_,{size:14})," Download"]}),e.jsxs("button",{className:"flex-1 flex flex-col items-center justify-center gap-1 bg-[#15151b] border border-[#2a2a35] hover:border-cyan-600 rounded-lg transition font-semibold text-xs py-2",children:[e.jsx(P,{size:14})," Share"]})]})]})]}),e.jsx("div",{className:"text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]",children:"Browser-based X3-Lang IDE with compile and testnet deployment capabilities."})]})}export{q as default};
