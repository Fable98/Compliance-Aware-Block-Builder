import Fastify from 'fastify';
import cors from '@fastify/cors';
import websocket from '@fastify/websocket';
import pg from 'pg';
import 'dotenv/config';

const { Pool } = pg;

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

const AI_EXPLAINER_URL = process.env.AI_EXPLAINER_URL || 'http://127.0.0.1:8000/explain';

const fastify = Fastify({ logger: true });

// Restrict CORS origins in production, permit localhost/dev origins by default
await fastify.register(cors, {
  origin: process.env.ALLOWED_ORIGINS
    ? process.env.ALLOWED_ORIGINS.split(',')
    : true,
});
await fastify.register(websocket);

const clients = new Set<any>();

fastify.get('/ws', { websocket: true }, (socket) => {
  clients.add(socket);
  fastify.log.info('Dashboard client connected');

  socket.on('close', () => {
    clients.delete(socket);
  });
});

function broadcast(data: unknown) {
  const payload = JSON.stringify(data);
  for (const client of clients) {
    if (client.readyState === 1) {
      client.send(payload);
    }
  }
}

fastify.get('/api/decisions', async (request, reply) => {
  const result = await pool.query(
    `SELECT tx_hash, sender, recipient, decision, risk_score, reason_codes, ai_explanation, created_at
     FROM compliance_decisions
     ORDER BY created_at DESC
     LIMIT 50`
  );
  return result.rows;
});

fastify.get('/api/blocks', async (request, reply) => {
  const result = await pool.query(
    `SELECT block_hash, block_number, builder_address, compliance_status, tx_count, created_at
     FROM blocks
     ORDER BY block_number DESC
     LIMIT 50`
  );
  return result.rows;
});

fastify.get('/api/stats', async (request, reply) => {
  const decisions = await pool.query(
    `SELECT decision, COUNT(*) FROM compliance_decisions GROUP BY decision`
  );
  const blocks = await pool.query(
    `SELECT compliance_status, COUNT(*) FROM blocks GROUP BY compliance_status`
  );
  return { decisions: decisions.rows, blocks: blocks.rows };
});

async function generateExplanation(row: {
  tx_hash: string;
  decision: string;
  risk_score: number;
  reason_codes: string[];
}) {
  try {
    const response = await fetch(AI_EXPLAINER_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        tx: row.tx_hash,
        decision: row.decision,
        risk_score: row.risk_score,
        reasons: row.reason_codes,
      }),
    });

    if (!response.ok) {
      fastify.log.error(`AI explainer returned ${response.status} for ${row.tx_hash}`);
      return;
    }

    const data = (await response.json()) as { narrative: string };

    await pool.query(
      `UPDATE compliance_decisions SET ai_explanation = $1 WHERE tx_hash = $2`,
      [data.narrative, row.tx_hash]
    );

    fastify.log.info(`AI explanation saved for ${row.tx_hash}`);
    broadcast({ type: 'explanation_ready', tx_hash: row.tx_hash, narrative: data.narrative });
  } catch (err) {
    fastify.log.error(`Failed to generate explanation for ${row.tx_hash}: ${err}`);
  }
}

let lastDecisionCount = 0;
setInterval(async () => {
  const countResult = await pool.query(`SELECT COUNT(*) FROM compliance_decisions`);
  const currentCount = parseInt(countResult.rows[0].count, 10);

  if (currentCount !== lastDecisionCount) {
    lastDecisionCount = currentCount;

    const result = await pool.query(
      `SELECT tx_hash, decision, risk_score, reason_codes, ai_explanation, created_at
       FROM compliance_decisions
       ORDER BY created_at DESC
       LIMIT 1`
    );

    if (result.rows.length > 0) {
      const latest = result.rows[0];
      broadcast({ type: 'new_decision', data: latest });

      if ((latest.decision === 'BLOCK' || latest.decision === 'FLAG') && !latest.ai_explanation) {
        generateExplanation(latest);
      }
    }
  }
}, 2000);

const port = Number(process.env.PORT) || 3002;

fastify.listen({ port, host: '0.0.0.0' }, (err) => {
  if (err) {
    fastify.log.error(err);
    process.exit(1);
  }
  console.log(`Fastify orchestration layer listening on http://localhost:${port}`);
});
