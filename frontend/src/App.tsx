// fix(#4): add integration guide for third-party attesters
import { useState } from 'react';

const API = {
  issueCredits: async (projectId: number, amount: number) => {
    // TODO: wire to Soroban RPC
    console.log(`Issuing ${amount} credits for project ${projectId}`);
  },
  retireCredits: async (projectId: number, amount: number, reason: string) => {
    console.log(`Retiring ${amount} credits from project ${projectId}: ${reason}`);
  },
};

export default function App() {
  const [tab, setTab] = useState<'browse' | 'retire'>('browse');
  const [reason, setReason] = useState('');
  const [amount, setAmount] = useState(1);

  return (
    <div className="min-h-screen bg-emerald-50 p-8">
      <header className="mb-8">
        <h1 className="text-3xl font-bold text-emerald-800">🌍 Stellar Carbon Registry</h1>
        <p className="text-emerald-600 mt-2">Tokenize, trade, and retire carbon credits on Stellar</p>
      </header>

      <nav className="flex gap-4 mb-6">
        <button
          onClick={() => setTab('browse')}
          className={`px-4 py-2 rounded-lg ${tab === 'browse' ? 'bg-emerald-600 text-white' : 'bg-white text-emerald-700'}`}
        >
          Browse Projects
        </button>
        <button
          onClick={() => setTab('retire')}
          className={`px-4 py-2 rounded-lg ${tab === 'retire' ? 'bg-emerald-600 text-white' : 'bg-white text-emerald-700'}`}
        >
          Retire Credits
        </button>
      </nav>

      {tab === 'browse' && (
        <div className="grid grid-cols-3 gap-4">
          {/* TODO: fetch and display projects from contract */}
          <div className="bg-white rounded-xl shadow p-4">
            <h3 className="font-semibold text-lg">Amazon Reforestation</h3>
            <p className="text-sm text-gray-500">Brazil · 2024 vintage</p>
            <p className="text-2xl font-bold text-emerald-700 mt-2">2.50 USDC</p>
            <button className="mt-3 w-full bg-emerald-600 text-white py-2 rounded-lg hover:bg-emerald-700">
              Buy Credits
            </button>
          </div>
        </div>
      )}

      {tab === 'retire' && (
        <div className="bg-white rounded-xl shadow p-6 max-w-lg">
          <h3 className="font-semibold text-lg mb-4">Retire Carbon Credits</h3>
          <label className="block text-sm text-gray-600 mb-1">Amount</label>
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(Number(e.target.value))}
            className="w-full border rounded-lg p-2 mb-3"
          />
          <label className="block text-sm text-gray-600 mb-1">Reason</label>
          <textarea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            className="w-full border rounded-lg p-2 mb-3"
            rows={3}
            placeholder="e.g., Offset Q3 2024 corporate emissions"
          />
          <button
            onClick={() => API.retireCredits(1, amount, reason)}
            className="w-full bg-red-600 text-white py-2 rounded-lg hover:bg-red-700"
          >
            Retire Credits Permanently
          </button>
        </div>
      )}
    </div>
  );
}
