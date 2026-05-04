// Chain config for V3 LP backtesting. Each chain has its own block
// time, native gas token, default Uniswap V3 subgraph URL, and a
// public RPC default for chain-head reads. Tier 2 wires this into
// the auto-run pipeline; tier 1 only uses the chainId enum to scope
// pool presets.

export type ChainId =
  | "ethereum"
  | "arbitrum"
  | "optimism"
  | "base"
  | "polygon";

export interface ChainConfig {
  id: ChainId;
  label: string;
  /** Average block time in seconds. Drives the grid runner's
   *  blocks_per_day calculation. */
  blockTimeSeconds: number;
  /** Native gas-paying token symbol, e.g. ETH, MATIC. */
  nativeTokenSymbol: string;
  /** DefiLlama coin id for native-token USD pricing, e.g.
   *  "coingecko:ethereum". */
  nativeTokenCoingeckoId: string;
  /** Free public RPC for chain-head reads when the user has no
   *  Alchemy key. Picked for high reliability + permissive CORS. */
  publicRpcUrl: string;
  /** Approx blocks per day, derived from blockTimeSeconds. */
  approxBlocksPerDay: number;
}

export const CHAIN_CONFIGS: Record<ChainId, ChainConfig> = {
  ethereum: {
    id: "ethereum",
    label: "Ethereum",
    blockTimeSeconds: 12,
    nativeTokenSymbol: "ETH",
    nativeTokenCoingeckoId: "coingecko:ethereum",
    publicRpcUrl: "https://eth.llamarpc.com",
    approxBlocksPerDay: 7200,
  },
  arbitrum: {
    id: "arbitrum",
    label: "Arbitrum",
    blockTimeSeconds: 0.25,
    nativeTokenSymbol: "ETH",
    nativeTokenCoingeckoId: "coingecko:ethereum",
    publicRpcUrl: "https://arb1.arbitrum.io/rpc",
    approxBlocksPerDay: 345_600,
  },
  optimism: {
    id: "optimism",
    label: "Optimism",
    blockTimeSeconds: 2,
    nativeTokenSymbol: "ETH",
    nativeTokenCoingeckoId: "coingecko:ethereum",
    publicRpcUrl: "https://mainnet.optimism.io",
    approxBlocksPerDay: 43_200,
  },
  base: {
    id: "base",
    label: "Base",
    blockTimeSeconds: 2,
    nativeTokenSymbol: "ETH",
    nativeTokenCoingeckoId: "coingecko:ethereum",
    publicRpcUrl: "https://mainnet.base.org",
    approxBlocksPerDay: 43_200,
  },
  polygon: {
    id: "polygon",
    label: "Polygon",
    blockTimeSeconds: 2.2,
    nativeTokenSymbol: "MATIC",
    nativeTokenCoingeckoId: "coingecko:matic-network",
    publicRpcUrl: "https://polygon-rpc.com",
    approxBlocksPerDay: 39_273,
  },
};

export const CHAIN_LIST: ChainConfig[] = Object.values(CHAIN_CONFIGS);
