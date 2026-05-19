/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_RPC_WS?: string;
  readonly VITE_RPC_WS_LOCAL?: string;
  readonly VITE_RPC_HTTP?: string;
  readonly VITE_RPC_HTTP_LOCAL?: string;
  readonly VITE_X3_TREASURY_ADDRESS?: string;
  readonly VITE_ETH_TREASURY_ADDRESS?: string;
  readonly VITE_SOL_TREASURY_ADDRESS?: string;
  readonly VITE_BSC_TREASURY_ADDRESS?: string;
  readonly VITE_POLYGON_TREASURY_ADDRESS?: string;
  readonly VITE_ARB_TREASURY_ADDRESS?: string;
  readonly VITE_AVAX_TREASURY_ADDRESS?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
