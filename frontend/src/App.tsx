import { useEffect, useState } from "react";
import {
  connectWallet,
  disconnectWallet,
  getActiveAddress,
  getFreighterNetwork,
} from "./wallet";
import { getXlmBalance, NETWORK_NAME } from "./stellar";
import { REGISTRY_CONTRACT_ID } from "./config";
import {
  getProject,
  balanceOf,
  buyCredits,
  retireCredits,
  getRetirement,
  type Project,
  type Retirement,
} from "./registry";

type Feedback =
  | { kind: "idle" }
  | { kind: "pending"; msg: string }
  | { kind: "success"; msg: string; hash?: string }
  | { kind: "error"; msg: string };

export default function App() {
  const [address, setAddress] = useState<string | null>(null);
  const [network, setNetwork] = useState<string | null>(null);
  const [balance, setBalance] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<Feedback>({ kind: "idle" });

  // Registry state
  const [projectId, setProjectId] = useState("1");
  const [project, setProject] = useState<Project | null>(null);
  const [creditBalance, setCreditBalance] = useState<bigint | null>(null);

  const [listingId, setListingId] = useState("1");
  const [buyAmount, setBuyAmount] = useState("1");

  const [retireAmount, setRetireAmount] = useState("1");
  const [retireReason, setRetireReason] = useState("Voluntary offset");
  const [lastCertId, setLastCertId] = useState<string>("");
  const [cert, setCert] = useState<Retirement | null>(null);

  useEffect(() => {
    (async () => {
      const a = await getActiveAddress();
      if (a) {
        setAddress(a);
        const net = await getFreighterNetwork();
        setNetwork(net?.network ?? null);
      }
    })();
  }, []);

  const refreshBalance = async (pub: string) => {
    try {
      setBalance(await getXlmBalance(pub));
    } catch {
      setBalance(null);
    }
  };

  const onConnect = async () => {
    setFeedback({ kind: "pending", msg: "Requesting Freighter access…" });
    const res = await connectWallet();
    if (res.error) return setFeedback({ kind: "error", msg: res.error });
    setAddress(res.address!);
    const net = await getFreighterNetwork();
    setNetwork(net?.network ?? null);
    setFeedback({ kind: "idle" });
    await refreshBalance(res.address!);
  };

  const onDisconnect = async () => {
    await disconnectWallet();
    setAddress(null);
    setNetwork(null);
    setBalance(null);
    setProject(null);
    setCreditBalance(null);
    setFeedback({ kind: "idle" });
  };

  const onLoadProject = async () => {
    setFeedback({ kind: "pending", msg: `Loading project #${projectId}…` });
    try {
      const p = await getProject(Number(projectId));
      setProject(p);
      if (address) setCreditBalance(await balanceOf(address, Number(projectId)));
      setFeedback({ kind: "idle" });
    } catch (e: any) {
      setProject(null);
      setFeedback({ kind: "error", msg: e.message ?? String(e) });
    }
  };

  const onBuy = async () => {
    if (!address) return;
    setFeedback({ kind: "pending", msg: "Signing & submitting purchase…" });
    try {
      const hash = await buyCredits(address, Number(listingId), Number(buyAmount));
      setFeedback({ kind: "success", msg: "Credits purchased", hash });
      await onLoadProject();
    } catch (e: any) {
      setFeedback({ kind: "error", msg: e.message ?? String(e) });
    }
  };

  const onRetire = async () => {
    if (!address) return;
    setFeedback({ kind: "pending", msg: "Signing & submitting retirement…" });
    try {
      const hash = await retireCredits(
        address,
        Number(projectId),
        Number(retireAmount),
        retireReason
      );
      setFeedback({ kind: "success", msg: "Credits retired — certificate issued", hash });
      await onLoadProject();
    } catch (e: any) {
      setFeedback({ kind: "error", msg: e.message ?? String(e) });
    }
  };

  const onLoadCert = async () => {
    if (!lastCertId) return;
    try {
      setCert(await getRetirement(Number(lastCertId)));
    } catch (e: any) {
      setCert(null);
      setFeedback({ kind: "error", msg: e.message ?? String(e) });
    }
  };

  const onTestnet = network === NETWORK_NAME;

  return (
    <div className="app">
      <header>
        <h1>🌍 Stellar Carbon Registry</h1>
        <p className="muted">
          Tokenize, trade, and retire carbon credits on Soroban · Network:{" "}
          <strong>{network ?? "—"}</strong> (required: {NETWORK_NAME})
        </p>
      </header>

      <section className="card">
        <h2>Wallet</h2>
        {!address ? (
          <button className="primary" onClick={onConnect}>Connect Freighter</button>
        ) : (
          <div>
            <div className="row"><span>Address:</span><code>{address}</code></div>
            <div className="row"><span>XLM:</span><strong>{balance ?? "—"}</strong></div>
            {!onTestnet && (
              <p className="warn">Switch Freighter to {NETWORK_NAME}.</p>
            )}
            <div className="actions"><button onClick={onDisconnect}>Disconnect</button></div>
          </div>
        )}
      </section>

      {!REGISTRY_CONTRACT_ID && (
        <section className="card">
          <p className="warn">
            No registry contract configured. Set <code>VITE_REGISTRY_CONTRACT_ID</code> in{" "}
            <code>frontend/.env</code> (see <code>.env.example</code>) to enable the panels below.
          </p>
        </section>
      )}

      {/* Browse a project */}
      <section className="card">
        <h2>Project explorer</h2>
        <div className="form">
          <label>Project ID
            <input value={projectId} onChange={(e) => setProjectId(e.target.value)} type="number" min="1" />
          </label>
          <button className="primary" onClick={onLoadProject}>Load project</button>
        </div>
        {project && (
          <div style={{ marginTop: 12 }}>
            <div className="row"><span>Name:</span><strong>{project.name}</strong></div>
            <div className="row"><span>Region:</span><span>{project.region}</span></div>
            <div className="row"><span>Type:</span><span>{project.project_type}</span></div>
            <div className="row"><span>Vintage:</span><span>{project.vintage}</span></div>
            <div className="row"><span>Issued:</span><span>{project.total_issued.toString()}</span></div>
            <div className="row"><span>Available:</span><span>{project.available.toString()}</span></div>
            <div className="row"><span>Retired:</span><span>{project.retired.toString()}</span></div>
            {creditBalance !== null && (
              <div className="row big"><span>Your credits:</span><strong>{creditBalance.toString()}</strong></div>
            )}
          </div>
        )}
      </section>

      {/* Buy */}
      <section className="card">
        <h2>Buy credits</h2>
        {address ? (
          <div className="form">
            <label>Listing ID
              <input value={listingId} onChange={(e) => setListingId(e.target.value)} type="number" min="1" />
            </label>
            <label>Amount
              <input value={buyAmount} onChange={(e) => setBuyAmount(e.target.value)} type="number" min="1" />
            </label>
            <button className="primary" onClick={onBuy}>Buy (pays USDC)</button>
          </div>
        ) : <p className="muted">Connect a wallet to buy credits.</p>}
      </section>

      {/* Retire */}
      <section className="card">
        <h2>Retire credits</h2>
        {address ? (
          <div className="form">
            <label>Project ID
              <input value={projectId} onChange={(e) => setProjectId(e.target.value)} type="number" min="1" />
            </label>
            <label>Amount
              <input value={retireAmount} onChange={(e) => setRetireAmount(e.target.value)} type="number" min="1" />
            </label>
            <label>Reason
              <input value={retireReason} onChange={(e) => setRetireReason(e.target.value)} />
            </label>
            <button className="primary" onClick={onRetire}>Retire (permanent)</button>
          </div>
        ) : <p className="muted">Connect a wallet to retire credits.</p>}
      </section>

      {/* Certificate viewer */}
      <section className="card">
        <h2>Retirement certificate</h2>
        <div className="form">
          <label>Certificate ID
            <input value={lastCertId} onChange={(e) => setLastCertId(e.target.value)} type="number" min="1" placeholder="e.g. 1" />
          </label>
          <button onClick={onLoadCert}>View certificate</button>
        </div>
        {cert && (
          <div style={{ marginTop: 12 }}>
            <div className="row"><span>Retiree:</span><code>{cert.retiree}</code></div>
            <div className="row"><span>Project:</span><span>#{cert.project_id.toString()}</span></div>
            <div className="row"><span>Amount:</span><strong>{cert.amount.toString()}</strong></div>
            <div className="row"><span>Reason:</span><span>{cert.reason}</span></div>
            <div className="row"><span>Timestamp:</span><span>{cert.timestamp.toString()}</span></div>
          </div>
        )}
      </section>

      {/* Feedback */}
      <section className="card">
        <h2>Status</h2>
        {feedback.kind === "idle" && <p className="muted">Ready.</p>}
        {feedback.kind === "pending" && <p className="info">{feedback.msg}</p>}
        {feedback.kind === "success" && (
          <p className="ok">
            ✅ {feedback.msg}
            {feedback.hash && (
              <> — <a href={`https://stellar.expert/explorer/testnet/tx/${feedback.hash}`} target="_blank" rel="noreferrer"><code>{feedback.hash.slice(0, 10)}…</code></a></>
            )}
          </p>
        )}
        {feedback.kind === "error" && <p className="err">❌ {feedback.msg}</p>}
      </section>
    </div>
  );
}
