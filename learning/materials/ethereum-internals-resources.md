# Ethereum Internals Resources

Resources for understanding the Ethereum platform underneath Aurix — the EVM, gas mechanics, transaction lifecycle, JSON-RPC, ABI encoding. Useful for everyone but essential for Vector B.

## Why It Matters For This Repo

Aurix talks directly to Ethereum via raw JSON-RPC and hand-crafted ABI. Understanding these layers is what enables the "no ethers-rs" decision — and what would make Vector B's mempool watching or Vector A's `eth_getLogs` ingestion possible.

## Primary Sources

### "Mastering Ethereum" — Andreas Antonopoulos & Gavin Wood

- URL: open-source on GitHub at `https://github.com/ethereumbook/ethereumbook`
- Format: free book (~400 pages)
- Difficulty: Moderate
- What you'll learn: end-to-end Ethereum mechanics — accounts, transactions, contracts, EVM, gas, mempool, nodes, smart contract development

The canonical free reference. Chapters 6 (transactions), 7 (smart contracts), 13 (EVM), and 14 (consensus) are most relevant for Aurix.

Caveat: written before the Merge, so some material on proof-of-work mining is now historical. Ignore those sections; the EVM and transaction mechanics are unchanged.

### Ethereum Yellow Paper

- URL: `https://ethereum.github.io/yellowpaper/paper.pdf`
- Length: ~35 pages
- Difficulty: Very high (formal specification with mathematical notation)
- What you'll learn: the precise specification of the EVM and consensus rules

Reference-grade. You don't read the Yellow Paper for understanding; you reference it when you need the exact answer to "what does opcode X actually do?" Useful for Vector B's calldata decoder if you encounter unusual opcodes.

### Ethereum.org Documentation

- URL: `https://ethereum.org/en/developers/docs/`
- Format: structured documentation site
- Difficulty: Moderate

The official docs are excellent. Particularly useful sections:
- Transactions and accounts model
- EVM and gas
- JSON-RPC API reference
- ABI specification

For Vector B, the `eth_subscribe` and `eth_call` documentation pages are mandatory reading.

## Specifications

### Solidity ABI Specification

- URL: `https://docs.soliditylang.org/en/latest/abi-spec.html`
- Length: ~20 pages
- Difficulty: Moderate
- What you'll learn: the exact byte-level encoding of function calls and return values

This is what enables Aurix's hand-crafted ABI. Every function selector, every padded address, every uint256 encoding is specified here. Read once carefully; reference when implementing new decoders.

### Ethereum JSON-RPC API Specification

- URL: `https://ethereum.org/en/developers/docs/apis/json-rpc/`
- Format: API reference
- What you'll learn: every JSON-RPC method, its parameters, and its return shape

For Aurix, the relevant methods are: `eth_call`, `eth_gasPrice`, `eth_getLogs`, `eth_subscribe`, `eth_getTransactionByHash`. Know each one's signature.

### EIP Specifications

- URL: `https://eips.ethereum.org`
- Format: list of Ethereum Improvement Proposals
- What you'll learn: specific changes and their rationale

Particularly relevant EIPs for Aurix:
- **EIP-20**: ERC-20 token standard
- **EIP-1559**: gas market mechanism (base fee + priority fee)
- **EIP-2930**: optional access lists
- **EIP-4844**: blob transactions (relevant for L2 awareness)

## Tools

### Etherscan

- URL: `https://etherscan.io`
- Format: blockchain explorer
- What you'll learn: real on-chain transactions, contract calls, token transfers

Useful for verifying claims. If Aurix's V3 5bps decode produces a price, you can verify against the actual `slot0()` value on Etherscan by viewing the pool contract and reading the slot0 function.

### Tenderly

- URL: `https://tenderly.co`
- Format: developer tooling for transaction simulation and debugging
- What you'll learn: how a specific transaction would execute, with EVM-level traces

For Vector B's calldata decoder, Tenderly is invaluable for understanding what a complex pending tx would actually do.

### evm.codes

- URL: `https://www.evm.codes`
- Format: opcode reference and interactive playground
- What you'll learn: every EVM opcode, its gas cost, its semantics

Reference-grade. Useful when you need to know exactly what an opcode does without reading the Yellow Paper.

## Topic-Specific

### Ethereum Account Model

- Mastering Ethereum chapter 4
- Understand: EOA vs contract account, nonce, balance, storage tree, code hash

### Transaction Lifecycle

- Mastering Ethereum chapter 6 + EIP-1559
- Understand: signing, submission, mempool propagation, block inclusion, finality

### EVM Execution

- Mastering Ethereum chapter 13 + Yellow Paper
- Understand: stack-based execution, gas metering, storage vs memory vs calldata

### Mempool and P2P

- Mastering Ethereum chapter 8
- Recent papers on private orderflow (covered in `mev-resources.md`)

### Layer 2 (Optional Background)

- Optimism, Arbitrum, Base documentation
- For Aurix's current scope, this is out-of-scope; useful for understanding where the ecosystem is heading

## When To Read What

**For Aurix's current code**: ABI spec + JSON-RPC reference for `eth_call` and `eth_gasPrice`. ~3-5 hours total.

**For Vector A (eth_getLogs ingestion)**: add the `eth_getLogs` reference + log filter docs + reorg-handling discussion. ~2-3 hours.

**For Vector B (mempool watching)**: add `eth_subscribe` reference + Mastering Ethereum chapter 6 (transactions) + chapter 8 (mempool). ~5-8 hours.

**For interview fluency**: Mastering Ethereum chapters 4-6 give the foundations of EOAs, transactions, and gas. ~5 hours.

## Related Files

- `concepts/foundations/tokens-and-the-defi-money-stack.md` — ERC-20 in context
- `concepts/domain-patterns/gas-and-execution-costs.md` — gas mechanics
- `concepts/domain-patterns/the-mempool-public-vs-private.md` — mempool dynamics
- `project/decisions/no-ethers-rs-handcrafted-abi.md` — why we touch this material directly
