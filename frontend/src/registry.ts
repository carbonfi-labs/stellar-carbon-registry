// Carbon Registry contract client.
//
// Wraps the Soroban contract in typed helpers the UI can call. Read-only calls
// (get_project, balance_of, …) are simulated against the RPC server and never
// need signing. State-changing calls (buy_credits, retire_credits, …) are
// assembled, simulated, signed with Freighter, and submitted.
import {
  Contract,
  TransactionBuilder,
  Account,
  Address,
  nativeToScVal,
  scValToNative,
  rpc,
  xdr,
} from "stellar-sdk";
import { NETWORK_PASSPHRASE, SOROBAN_RPC_URL, REGISTRY_CONTRACT_ID } from "./config";
import { signWithFreighter } from "./wallet";

const server = new rpc.Server(SOROBAN_RPC_URL, {
  allowHttp: SOROBAN_RPC_URL.startsWith("http://"),
});

// ---- value helpers -------------------------------------------------------

const u64 = (n: number | bigint) => nativeToScVal(BigInt(n), { type: "u64" });
const u128 = (n: number | bigint) => nativeToScVal(BigInt(n), { type: "u128" });
const u32 = (n: number) => nativeToScVal(n, { type: "u32" });
const str = (s: string) => nativeToScVal(s, { type: "string" });
const addr = (a: string) => new Address(a).toScVal();

export type Project = {
  owner: string;
  name: string;
  region: string;
  project_type: string;
  vintage: number;
  total_issued: bigint;
  available: bigint;
  retired: bigint;
};

export type Listing = {
  project_id: bigint;
  seller: string;
  price_per_credit: bigint;
  remaining: bigint;
  active: boolean;
};

export type Retirement = {
  retiree: string;
  project_id: bigint;
  amount: bigint;
  reason: string;
  timestamp: bigint;
};

function contract(): Contract {
  if (!REGISTRY_CONTRACT_ID) {
    throw new Error(
      "Registry contract id not configured. Set VITE_REGISTRY_CONTRACT_ID in your .env."
    );
  }
  return new Contract(REGISTRY_CONTRACT_ID);
}

// A well-formed but throwaway source account used only to build read-only
// simulations. Soroban RPC `simulateTransaction` never checks the source
// account's balance or sequence for a read, so this need not exist on-chain.
const NULL_ACCOUNT = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

/** Simulate a read-only method and decode its return value to a JS native. */
async function simulateRead<T>(method: string, args: xdr.ScVal[]): Promise<T> {
  const c = contract();
  const source = new Account(NULL_ACCOUNT, "0");
  const tx = new TransactionBuilder(source, {
    fee: "100",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(c.call(method, ...args))
    .setTimeout(30)
    .build();

  const sim = await server.simulateTransaction(tx);
  if (rpc.Api.isSimulationError(sim)) {
    throw new Error(`Simulation failed for ${method}: ${sim.error}`);
  }
  const retval = sim.result?.retval;
  if (!retval) throw new Error(`No return value from ${method}`);
  return scValToNative(retval) as T;
}

/**
 * Build, simulate, sign (Freighter) and submit a state-changing call.
 * Returns the transaction hash once the ledger has applied it.
 */
async function invoke(
  publicKey: string,
  method: string,
  args: xdr.ScVal[]
): Promise<string> {
  const c = contract();
  const source = await server.getAccount(publicKey);
  const built = new TransactionBuilder(source, {
    fee: "1000000",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(c.call(method, ...args))
    .setTimeout(120)
    .build();

  const prepared = await server.prepareTransaction(built);
  const signed = await signWithFreighter(prepared.toXDR(), NETWORK_PASSPHRASE);
  if (signed.error) throw new Error(signed.error);

  const tx = TransactionBuilder.fromXDR(signed.signedTxXdr!, NETWORK_PASSPHRASE);
  const sent = await server.sendTransaction(tx);
  if (sent.status === "ERROR") {
    throw new Error(`Submission failed: ${JSON.stringify(sent.errorResult)}`);
  }

  // Poll until the transaction is confirmed.
  let status = await server.getTransaction(sent.hash);
  const deadline = Date.now() + 30_000;
  while (status.status === "NOT_FOUND" && Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 1500));
    status = await server.getTransaction(sent.hash);
  }
  if (status.status !== "SUCCESS") {
    throw new Error(`Transaction ${sent.hash} did not succeed: ${status.status}`);
  }
  return sent.hash;
}

// ---- reads ---------------------------------------------------------------

export const getProject = (id: number | bigint) =>
  simulateRead<Project>("get_project", [u64(id)]);

export const getListing = (id: number | bigint) =>
  simulateRead<Listing>("get_listing", [u64(id)]);

export const getRetirement = (id: number | bigint) =>
  simulateRead<Retirement>("get_retirement", [u64(id)]);

export const balanceOf = (holder: string, projectId: number | bigint) =>
  simulateRead<bigint>("balance_of", [addr(holder), u64(projectId)]);

// ---- writes --------------------------------------------------------------

export const buyCredits = (
  publicKey: string,
  listingId: number | bigint,
  amount: number | bigint
) => invoke(publicKey, "buy_credits", [addr(publicKey), u64(listingId), u128(amount)]);

export const retireCredits = (
  publicKey: string,
  projectId: number | bigint,
  amount: number | bigint,
  reason: string
) =>
  invoke(publicKey, "retire_credits", [
    addr(publicKey),
    u64(projectId),
    u128(amount),
    str(reason),
  ]);

export const listCredits = (
  publicKey: string,
  projectId: number | bigint,
  pricePerCredit: number | bigint,
  amount: number | bigint
) =>
  invoke(publicKey, "list_credits", [
    u64(projectId),
    u128(pricePerCredit),
    u128(amount),
  ]);

// Kept for completeness; verifier/admin-gated methods.
export const createProject = (
  publicKey: string,
  name: string,
  region: string,
  projectType: string,
  vintage: number
) =>
  invoke(publicKey, "create_project", [
    str(name),
    str(region),
    str(projectType),
    u32(vintage),
  ]);

export const issueCredits = (
  publicKey: string,
  projectId: number | bigint,
  amount: number | bigint
) => invoke(publicKey, "issue_credits", [u64(projectId), u128(amount)]);
