// Curated list of popular Uniswap V3 pools across the supported chains.
// Each entry pairs a pool address with display metadata so the
// settings panel can show "WETH/USDC 0.05% (Ethereum)" instead of an
// opaque 0x-address. Decimals + token symbols are fetched at runtime
// via `lp_pool_metadata` once the user picks one — these constants
// are just for the picker UI.

import type { ChainId } from "./chains";

export interface PoolPreset {
  /** Stable id for the picker. */
  id: string;
  /** Pool contract address on the chain. */
  address: string;
  chainId: ChainId;
  protocol: "uniswap-v3" | "sushiswap-v3" | "pancakeswap-v3";
  /** Display label, e.g. "WETH/USDC 0.05%". */
  label: string;
  /** Used to short-list "popular" presets in the UI. */
  popular?: boolean;
}

export const POOL_PRESETS: PoolPreset[] = [
  // ---- Ethereum mainnet ----
  {
    id: "uniswap-v3-ethereum-weth-usdc-5",
    address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
    chainId: "ethereum",
    protocol: "uniswap-v3",
    label: "WETH/USDC 0.05%",
    popular: true,
  },
  {
    id: "uniswap-v3-ethereum-weth-usdc-30",
    address: "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8",
    chainId: "ethereum",
    protocol: "uniswap-v3",
    label: "WETH/USDC 0.30%",
  },
  {
    id: "uniswap-v3-ethereum-weth-usdt-30",
    address: "0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36",
    chainId: "ethereum",
    protocol: "uniswap-v3",
    label: "WETH/USDT 0.30%",
    popular: true,
  },
  {
    id: "uniswap-v3-ethereum-wbtc-usdc-30",
    address: "0x99ac8cA7087fA4A2A1FB6357269965A2014ABc35",
    chainId: "ethereum",
    protocol: "uniswap-v3",
    label: "WBTC/USDC 0.30%",
    popular: true,
  },
  {
    id: "uniswap-v3-ethereum-usdc-usdt-1",
    address: "0x3416cF6C708Da44DB2624D63ea0AAef7113527C6",
    chainId: "ethereum",
    protocol: "uniswap-v3",
    label: "USDC/USDT 0.01%",
  },
  {
    id: "uniswap-v3-ethereum-dai-usdc-1",
    address: "0x5777d92f208679DB4b9778590Fa3CAB3aC9e2168",
    chainId: "ethereum",
    protocol: "uniswap-v3",
    label: "DAI/USDC 0.01%",
  },
  {
    id: "uniswap-v3-ethereum-wbtc-eth-30",
    address: "0xCBCdF9626bC03E24f779434178A73a0B4bad62eD",
    chainId: "ethereum",
    protocol: "uniswap-v3",
    label: "WBTC/WETH 0.30%",
    popular: true,
  },
  {
    id: "uniswap-v3-ethereum-link-eth-30",
    address: "0xa6Cc3C2531FdaA6Ae1A3CA84c2855806728693e8",
    chainId: "ethereum",
    protocol: "uniswap-v3",
    label: "LINK/WETH 0.30%",
  },
  // ---- Arbitrum ----
  {
    id: "uniswap-v3-arbitrum-weth-usdc-5",
    address: "0xC31E54c7a869B9FcBEcc14363CF510d1c41fa443",
    chainId: "arbitrum",
    protocol: "uniswap-v3",
    label: "WETH/USDC 0.05% (Arb)",
    popular: true,
  },
  {
    id: "uniswap-v3-arbitrum-arb-usdc-5",
    address: "0xb0F6cA40411360c03d41C5fFc5F179b8403CdcF8",
    chainId: "arbitrum",
    protocol: "uniswap-v3",
    label: "ARB/USDC 0.05%",
  },
  // ---- Optimism ----
  {
    id: "uniswap-v3-optimism-weth-usdc-5",
    address: "0x85149247691df622eaF1a8Bd0CaFd40BC45154a9",
    chainId: "optimism",
    protocol: "uniswap-v3",
    label: "WETH/USDC 0.05% (OP)",
    popular: true,
  },
  // ---- Base ----
  {
    id: "uniswap-v3-base-weth-usdc-5",
    address: "0xd0b53D9277642d899DF5C87A3966A349A798F224",
    chainId: "base",
    protocol: "uniswap-v3",
    label: "WETH/USDC 0.05% (Base)",
    popular: true,
  },
  // ---- Polygon ----
  {
    id: "uniswap-v3-polygon-wmatic-usdc-5",
    address: "0xA374094527e1673A86dE625aa59517c5dE346d32",
    chainId: "polygon",
    protocol: "uniswap-v3",
    label: "WMATIC/USDC 0.05%",
  },
  // ---- Sushi V3 ----
  {
    id: "sushi-v3-ethereum-weth-usdc-5",
    address: "0x35644Fb61afbc458bf92B15adD6ABc1996Be5014",
    chainId: "ethereum",
    protocol: "sushiswap-v3",
    label: "WETH/USDC 0.05%",
  },
  {
    id: "sushi-v3-arbitrum-weth-usdc-5",
    address: "0xf3eB87c1F6020982173C908E7eB31aA66c1f0296",
    chainId: "arbitrum",
    protocol: "sushiswap-v3",
    label: "WETH/USDC 0.05% (Arb)",
  },
  // ---- Pancake V3 ----
  {
    id: "pancake-v3-ethereum-weth-usdc-5",
    address: "0x6Ca298D2983aB03Aa1dA7679389D955A4eFEE15C",
    chainId: "ethereum",
    protocol: "pancakeswap-v3",
    label: "WETH/USDC 0.05%",
  },
  {
    id: "pancake-v3-arbitrum-weth-usdc-5",
    address: "0xd9e497DC4eC5e3aE74E3FCD2b9f1A0e0E5c20D8b",
    chainId: "arbitrum",
    protocol: "pancakeswap-v3",
    label: "WETH/USDC 0.05% (Arb)",
  },
];

export function findPoolPreset(address: string): PoolPreset | undefined {
  const lower = address.toLowerCase();
  return POOL_PRESETS.find((p) => p.address.toLowerCase() === lower);
}
