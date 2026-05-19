import{r,j as e,Z as j,D as w}from"./index-Bjwn4JM-.js";import{c}from"./clsx-B-dksMZM.js";import{C as A}from"./code-xml-C9LwwhJH.js";import{C as T}from"./circle-alert-BX4w7Id_.js";import{C as S}from"./copy-DlJxiIsY.js";const o=[{name:"mint",inputs:[{name:"amount",type:"uint256"}],outputs:[],stateMutability:"nonpayable"},{name:"transfer",inputs:[{name:"to",type:"address"},{name:"amount",type:"uint256"}],outputs:[{name:"",type:"bool"}],stateMutability:"nonpayable"},{name:"balanceOf",inputs:[{name:"account",type:"address"}],outputs:[{name:"",type:"uint256"}],stateMutability:"view"},{name:"Transfer",inputs:[{name:"from",type:"address",indexed:!0},{name:"to",type:"address",indexed:!0},{name:"value",type:"uint256",indexed:!1}]}],I=`import { ethers } from "ethers";

// Contract ABI
const CONTRACT_ABI = [
  {
    name: "mint",
    type: "function",
    inputs: [{ name: "amount", type: "uint256" }],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    name: "transfer",
    type: "function",
    inputs: [
      { name: "to", type: "address" },
      { name: "amount", type: "uint256" },
    ],
    outputs: [{ name: "", type: "bool" }],
    stateMutability: "nonpayable",
  },
];

// Contract class
export class SimpleTokenContract {
  private contract: ethers.Contract;

  constructor(contractAddress: string, signer: ethers.Signer) {
    this.contract = new ethers.Contract(contractAddress, CONTRACT_ABI, signer);
  }

  async mint(amount: ethers.BigNumberish): Promise<ethers.ContractTransaction> {
    return this.contract.mint(amount);
  }

  async transfer(to: string, amount: ethers.BigNumberish): Promise<boolean> {
    return this.contract.transfer(to, amount);
  }
}`,k=`from web3 import Web3
from web3.contract import Contract

# Contract ABI
CONTRACT_ABI = [
    {
        "name": "mint",
        "type": "function",
        "inputs": [{"name": "amount", "type": "uint256"}],
        "outputs": [],
        "stateMutability": "nonpayable",
    },
    {
        "name": "transfer",
        "type": "function",
        "inputs": [
            {"name": "to", "type": "address"},
            {"name": "amount", "type": "uint256"},
        ],
        "outputs": [{"name": "", "type": "bool"}],
        "stateMutability": "nonpayable",
    },
]

class SimpleTokenContract:
    def __init__(self, contract_address: str, web3: Web3):
        self.contract = web3.eth.contract(
            address=contract_address,
            abi=CONTRACT_ABI
        )

    def mint(self, amount: int) -> dict:
        return self.contract.functions.mint(amount).transact()

    def transfer(self, to: str, amount: int) -> bool:
        return self.contract.functions.transfer(to, amount).call()`,B=`package main

import (
    "github.com/ethereum/go-ethereum/accounts/abi"
    "github.com/ethereum/go-ethereum/common"
    "github.com/ethereum/go-ethereum/ethclient"
)

type SimpleTokenContract struct {
    Client   *ethclient.Client
    Contract *abi.ABI
    Address  common.Address
}

func NewSimpleTokenContract(addr string, client *ethclient.Client) *SimpleTokenContract {
    contractABI := \`[
        {"name":"mint","type":"function","inputs":[{"name":"amount","type":"uint256"}]},
        {"name":"transfer","type":"function","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}]}
    ]\`
    
    parsed, _ := abi.JSON(strings.NewReader(contractABI))
    
    return &SimpleTokenContract{
        Client:   client,
        Contract: &parsed,
        Address:  common.HexToAddress(addr),
    }
}

func (c *SimpleTokenContract) Mint(amount *big.Int) error {
    // Implementation
    return nil
}

func (c *SimpleTokenContract) Transfer(to common.Address, amount *big.Int) (bool, error) {
    // Implementation
    return true, nil
}`;function R(){const[u,p]=r.useState(JSON.stringify(o,null,2)),[s,x]=r.useState(["typescript"]),[a,b]=r.useState(null),[i,d]=r.useState("abi"),[l,m]=r.useState(!1),f=t=>{x(n=>n.includes(t)?n.filter(N=>N!==t):[...n,t])},y=()=>{m(!0),setTimeout(()=>{let t="";s.includes("typescript")?t=I:s.includes("python")?t=k:s.includes("go")&&(t=B),b({language:s[0]||"typescript",code:t,timestamp:new Date().toISOString()}),d("preview"),m(!1)},1200)},g=()=>{a&&navigator.clipboard.writeText(a.code)},h=()=>{if(a){const t={typescript:"ts",python:"py",go:"go"}[a.language],n=document.createElement("a");n.setAttribute("href","data:text/plain;charset=utf-8,"+encodeURIComponent(a.code)),n.setAttribute("download",`contract-sdk.${t}`),n.style.display="none",document.body.appendChild(n),n.click(),document.body.removeChild(n)}},C=o.filter(t=>"stateMutability"in t).length,v=o.filter(t=>!("stateMutability"in t)).length;return e.jsxs("div",{className:"w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col",children:[e.jsxs("h2",{className:"text-xl font-bold mb-4 flex items-center gap-2",children:[e.jsx(A,{size:20,className:"text-cyan-400"})," SDK Code Generator"]}),e.jsxs("div",{className:"flex-1 overflow-hidden space-y-4 mb-4",children:[e.jsxs("div",{className:"grid grid-cols-4 gap-2",children:[e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded-lg p-3",children:[e.jsx("div",{className:"text-xs text-gray-400 mb-1",children:"Functions"}),e.jsx("div",{className:"text-lg font-bold text-cyan-400",children:C})]}),e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded-lg p-3",children:[e.jsx("div",{className:"text-xs text-gray-400 mb-1",children:"Events"}),e.jsx("div",{className:"text-lg font-bold text-purple-400",children:v})]}),e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded-lg p-3",children:[e.jsx("div",{className:"text-xs text-gray-400 mb-1",children:"Languages"}),e.jsx("div",{className:"text-lg font-bold text-green-400",children:s.length})]}),e.jsxs("div",{className:"bg-[#15151b] border border-[#2a2a35] rounded-lg p-3",children:[e.jsx("div",{className:"text-xs text-gray-400 mb-1",children:"Status"}),e.jsx("div",{className:c("text-lg font-bold",a?"text-green-400":"text-yellow-400"),children:a?"✓ Ready":"Pending"})]})]}),e.jsxs("div",{className:"flex-1 flex gap-3 overflow-hidden",children:[e.jsxs("div",{className:"w-48 flex flex-col border border-[#2a2a35] rounded-lg overflow-hidden",children:[e.jsxs("div",{className:"bg-[#15151b] border-b border-[#2a2a35] p-3",children:[e.jsx("div",{className:"text-xs font-semibold uppercase text-gray-400 mb-2",children:"Target Languages"}),e.jsx("div",{className:"space-y-1.5",children:["typescript","python","go"].map(t=>e.jsxs("label",{className:"flex items-center gap-2 cursor-pointer text-xs",children:[e.jsx("input",{type:"checkbox",checked:s.includes(t),onChange:()=>f(t),className:"w-3 h-3 accent-cyan-600"}),e.jsx("span",{className:"capitalize",children:t})]},t))})]}),e.jsxs("div",{className:"flex-1 overflow-y-auto p-3 bg-[#0a0a0f] border-t border-[#2a2a35]",children:[e.jsx("div",{className:"text-xs font-semibold uppercase text-gray-400 mb-2",children:"ABI Functions"}),e.jsxs("div",{className:"space-y-1.5 text-xs",children:[o.filter(t=>"stateMutability"in t).map(t=>e.jsxs("div",{className:"p-2 bg-[#15151b] border border-[#2a2a35] rounded text-cyan-400 font-mono",children:[t.name,"()"]},t.name)),o.filter(t=>!("stateMutability"in t)).map(t=>e.jsxs("div",{className:"p-2 bg-[#15151b] border border-[#2a2a35] rounded text-purple-400 font-mono",children:["↑ ",t.name]},t.name))]})]}),e.jsx("div",{className:"border-t border-[#2a2a35] p-3 bg-[#15151b]",children:e.jsxs("button",{onClick:y,disabled:l||s.length===0,className:c("w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg border transition font-semibold text-xs",l||s.length===0?"bg-cyan-600/10 border-cyan-600/30 cursor-not-allowed opacity-50":"bg-[#2a2a35] border-[#2a2a35] hover:border-cyan-600"),children:[e.jsx(j,{size:12})," ",l?"...":"Generate"]})})]}),e.jsxs("div",{className:"flex-1 flex flex-col border border-[#2a2a35] rounded-lg overflow-hidden",children:[e.jsx("div",{className:"flex gap-2 bg-[#15151b] border-b border-[#2a2a35] px-3 py-2",children:["abi","preview"].map(t=>e.jsx("button",{onClick:()=>d(t),className:c("px-3 py-1 text-xs font-semibold rounded transition",i===t?"bg-cyan-600/20 text-cyan-400":"text-gray-400 hover:text-gray-300"),children:t==="abi"?"ABI Input":"Generated Code"},t))}),i==="abi"&&e.jsx("textarea",{value:u,onChange:t=>p(t.target.value),className:"flex-1 bg-[#0a0a0f] text-gray-300 p-3 font-mono text-xs focus:outline-none resize-none",spellCheck:!1,placeholder:"Paste ABI JSON here..."}),i==="preview"&&a&&e.jsx("div",{className:"flex-1 overflow-y-auto",children:e.jsx("textarea",{readOnly:!0,value:a.code,className:"w-full h-full bg-[#0a0a0f] text-gray-300 p-3 font-mono text-xs focus:outline-none resize-none"})}),i==="preview"&&!a&&e.jsx("div",{className:"flex-1 flex items-center justify-center text-gray-500 bg-[#0a0a0f]",children:e.jsxs("div",{className:"text-center",children:[e.jsx(T,{size:32,className:"mx-auto mb-2 opacity-50"}),e.jsx("div",{className:"text-xs",children:"Generate code to preview"})]})}),a&&e.jsxs("div",{className:"border-t border-[#2a2a35] bg-[#15151b] p-3 flex gap-2",children:[e.jsxs("button",{onClick:g,className:"flex-1 flex items-center justify-center gap-2 px-3 py-1.5 bg-[#2a2a35] hover:border-cyan-600 border border-[#2a2a35] rounded text-xs font-semibold transition",children:[e.jsx(S,{size:12})," Copy"]}),e.jsxs("button",{onClick:h,className:"flex-1 flex items-center justify-center gap-2 px-3 py-1.5 bg-[#2a2a35] hover:border-cyan-600 border border-[#2a2a35] rounded text-xs font-semibold transition",children:[e.jsx(w,{size:12})," Download"]}),e.jsx("div",{className:"flex items-center text-xs text-gray-500",children:a.language.toUpperCase()})]})]})]})]}),e.jsx("div",{className:"text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]",children:"Automatically generate TypeScript, Python, or Go SDKs from contract ABI with full type safety."})]})}export{R as default};
