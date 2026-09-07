// Network + contract configuration.
//
// The registry contract id is read from Vite env (`VITE_REGISTRY_CONTRACT_ID`)
// so the same build can target a locally-deployed contract, testnet, or
// mainnet without code changes. See `.env.example`.
import { Networks } from "stellar-sdk";

export const NETWORK_NAME = import.meta.env.VITE_NETWORK ?? "TESTNET";

export const NETWORK_PASSPHRASE =
  NETWORK_NAME === "PUBLIC" ? Networks.PUBLIC : Networks.TESTNET;

export const HORIZON_URL =
  import.meta.env.VITE_HORIZON_URL ??
  (NETWORK_NAME === "PUBLIC"
    ? "https://horizon.stellar.org"
    : "https://horizon-testnet.stellar.org");

export const SOROBAN_RPC_URL =
  import.meta.env.VITE_SOROBAN_RPC_URL ??
  (NETWORK_NAME === "PUBLIC"
    ? "https://mainnet.sorobanrpc.com"
    : "https://soroban-testnet.stellar.org");

export const REGISTRY_CONTRACT_ID: string =
  import.meta.env.VITE_REGISTRY_CONTRACT_ID ?? "";
