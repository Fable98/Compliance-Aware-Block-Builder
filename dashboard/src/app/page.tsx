'use client';

import { useEffect, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3002';

type Decision = {
  tx_hash: string;
  sender: string;
  recipient: string;
  decision: string;
  risk_score: number;
  reason_codes: string[];
  ai_explanation: string | null;
  created_at: string;
};

type Block = {
  block_hash: string;
  block_number: string;
  builder_address: string;
  compliance_status: string;
  tx_count: number;
  created_at: string;
};

type Stats = {
  decisions: { decision: string; count: string }[];
  blocks: { compliance_status: string; count: string }[];
};

function decisionColor(decision: string) {
  if (decision === 'BLOCK') return 'bg-red-600 text-white';
  if (decision === 'FLAG') return 'bg-yellow-500 text-black';
  return 'bg-green-600 text-white';
}

function blockStatusColor(status: string) {
  if (status === 'EXPOSED_EXTERNAL') return 'bg-red-600 text-white';
  if (status === 'UNATTRIBUTED') return 'bg-gray-500 text-white';
  return 'bg-green-600 text-white';
}

function shortAddr(addr: string) {
  if (!addr) return '';
  return `${addr.slice(0, 6)}...${addr.slice(-4)}`;
}

export default function Dashboard() {
  const [decisions, setDecisions] = useState<Decision[]>([]);
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [stats, setStats] = useState<Stats | null>(null);
  const [selected, setSelected] = useState<Decision | null>(null);

  async function fetchAll() {
    const [d, b, s] = await Promise.all([
      fetch(`${API_URL}/api/decisions`).then((r) => r.json()),
      fetch(`${API_URL}/api/blocks`).then((r) => r.json()),
      fetch(`${API_URL}/api/stats`).then((r) => r.json()),
    ]);
    setDecisions(d);
    setBlocks(b);
    setStats(s);
  }

  useEffect(() => {
    fetchAll();

    // Live updates via WebSocket
    const ws = new WebSocket(`ws://localhost:3002/ws`);
    ws.onmessage = () => {
      fetchAll();
    };

    // Fallback polling in case WS drops
    const interval = setInterval(fetchAll, 5000);

    return () => {
      ws.close();
      clearInterval(interval);
    };
  }, []);

  const allowCount = stats?.decisions.find((d) => d.decision === 'ALLOW')?.count ?? '0';
  const flagCount = stats?.decisions.find((d) => d.decision === 'FLAG')?.count ?? '0';
  const blockCount = stats?.decisions.find((d) => d.decision === 'BLOCK')?.count ?? '0';
  const exposedBlocks = stats?.blocks.find((b) => b.compliance_status === 'EXPOSED_EXTERNAL')?.count ?? '0';

  return (
    <main className="min-h-screen bg-neutral-950 text-white p-8">
      <h1 className="text-2xl font-bold mb-1 text-white">Compliance-Aware Block Builder</h1>
      <p className="text-neutral-400 mb-6">Pre-execution screening & post-execution proposer attribution</p>

      <div className="grid grid-cols-4 gap-4 mb-8">
        <Card className="bg-neutral-900 border-neutral-800 text-white">
          <CardHeader><CardTitle className="text-neutral-300 text-sm">Allowed Transactions</CardTitle></CardHeader>
          <CardContent><p className="text-3xl font-bold text-green-500">{allowCount}</p></CardContent>
        </Card>
        <Card className="bg-neutral-900 border-neutral-800 text-white">
          <CardHeader><CardTitle className="text-neutral-300 text-sm">Flagged (Indirect Risk)</CardTitle></CardHeader>
          <CardContent><p className="text-3xl font-bold text-yellow-500">{flagCount}</p></CardContent>
        </Card>
        <Card className="bg-neutral-900 border-neutral-800 text-white">
          <CardHeader><CardTitle className="text-neutral-300 text-sm">Blocked Transactions</CardTitle></CardHeader>
          <CardContent><p className="text-3xl font-bold text-red-500">{blockCount}</p></CardContent>
        </Card>
        <Card className="bg-neutral-900 border-neutral-800 text-white">
          <CardHeader><CardTitle className="text-neutral-300 text-sm">Exposed Blocks (Sanctioned Proposer)</CardTitle></CardHeader>
          <CardContent><p className="text-3xl font-bold text-red-500">{exposedBlocks}</p></CardContent>
        </Card>
      </div>

      <Tabs defaultValue="mempool" className="w-full">
        <TabsList className="bg-neutral-900 border border-neutral-800 text-neutral-400">
          <TabsTrigger value="mempool" className="data-[state=active]:bg-neutral-800 data-[state=active]:text-white">Live Mempool Monitor</TabsTrigger>
          <TabsTrigger value="blocks" className="data-[state=active]:bg-neutral-800 data-[state=active]:text-white">Block Builder Telemetry</TabsTrigger>
          <TabsTrigger value="lineage" className="data-[state=active]:bg-neutral-800 data-[state=active]:text-white">Lineage & Audit Inspector</TabsTrigger>
        </TabsList>

        <TabsContent value="mempool">
          <Card className="bg-neutral-900 border-neutral-800 text-white">
            <CardContent className="pt-6">
              <Table>
                <TableHeader>
                  <TableRow className="border-neutral-800 hover:bg-transparent">
                    <TableHead className="text-neutral-300 font-medium">Tx Hash</TableHead>
                    <TableHead className="text-neutral-300 font-medium">Sender</TableHead>
                    <TableHead className="text-neutral-300 font-medium">Recipient</TableHead>
                    <TableHead className="text-neutral-300 font-medium">Decision</TableHead>
                    <TableHead className="text-neutral-300 font-medium">Risk Score</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {decisions.map((d) => (
                    <TableRow
                      key={d.tx_hash}
                      className="cursor-pointer hover:bg-neutral-800/80 border-neutral-800 text-white"
                      onClick={() => setSelected(d)}
                    >
                      <TableCell className="font-mono text-sm text-white">{shortAddr(d.tx_hash)}</TableCell>
                      <TableCell className="font-mono text-sm text-white">{shortAddr(d.sender)}</TableCell>
                      <TableCell className="font-mono text-sm text-white">{shortAddr(d.recipient)}</TableCell>
                      <TableCell><Badge className={decisionColor(d.decision)}>{d.decision}</Badge></TableCell>
                      <TableCell className="text-white">{d.risk_score}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="blocks">
          <Card className="bg-neutral-900 border-neutral-800 text-white">
            <CardContent className="pt-6">
              <Table>
                <TableHeader>
                  <TableRow className="border-neutral-800 hover:bg-transparent">
                    <TableHead className="text-neutral-300 font-medium">Block #</TableHead>
                    <TableHead className="text-neutral-300 font-medium">Proposer / Builder</TableHead>
                    <TableHead className="text-neutral-300 font-medium">Tx Count</TableHead>
                    <TableHead className="text-neutral-300 font-medium">Status</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {blocks.map((b) => (
                    <TableRow key={b.block_hash} className="border-neutral-800 text-white">
                      <TableCell className="text-white">{b.block_number}</TableCell>
                      <TableCell className="font-mono text-sm text-white">{shortAddr(b.builder_address)}</TableCell>
                      <TableCell className="text-white">{b.tx_count}</TableCell>
                      <TableCell><Badge className={blockStatusColor(b.compliance_status)}>{b.compliance_status}</Badge></TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="lineage">
          <Card className="bg-neutral-900 border-neutral-800 text-white">
            <CardContent className="pt-6">
              {selected ? (
                <div className="space-y-3">
                  <p className="text-neutral-300 text-sm">Selected transaction lineage</p>
                  <div className="font-mono text-sm space-y-2">
                    <p><span className="text-neutral-400">Tx Hash:</span> <span className="text-white">{selected.tx_hash}</span></p>
                    <p><span className="text-neutral-400">Sender:</span> <span className="text-white">{selected.sender}</span></p>
                    <p><span className="text-neutral-400">Recipient:</span> <span className="text-white">{selected.recipient}</span></p>
                    <p><span className="text-neutral-400">Decision:</span> <Badge className={decisionColor(selected.decision)}>{selected.decision}</Badge></p>
                    <p><span className="text-neutral-400">Risk Score:</span> <span className="text-white">{selected.risk_score}</span></p>
                    <p><span className="text-neutral-400">Reason Codes:</span> <span className="text-white">{selected.reason_codes.join(', ') || 'None'}</span></p>
                  </div>
                  {selected.ai_explanation && (
                    <div className="border-t border-neutral-800 pt-4 mt-4">
                      <p className="text-neutral-400 text-sm mb-2">AI-Generated Compliance Narrative</p>
                      <p className="text-neutral-200 text-sm leading-relaxed">{selected.ai_explanation}</p>
                    </div>
                  )}
                  <div className="border-t border-neutral-800 pt-4 mt-4">
                    <p className="text-neutral-400 text-sm mb-2">Pipeline: Ingestion → Sanctions Lookup → Entity Attribution → Policy Decision → Builder Action</p>
                  </div>
                </div>
              ) : (
                <p className="text-neutral-400">Click a transaction in the Live Mempool Monitor tab to inspect its lineage.</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </main>
  );
}
