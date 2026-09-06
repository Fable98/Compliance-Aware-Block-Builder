import Fastify from 'fastify';
import cors from '@fastify/cors';
import websocket from '@fastify/websocket';
import pg from 'pg';
import 'dotenv/config';

const { Pool } = pg;

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

const fastify = Fastify({ logger: true });

await fastify.register(cors, { origin: true });
await fastify.register(websocket);

// Track connected WebSocket clients so we can broadcast updates
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

// REST: recent compliance decisions (pre-execution)
fastify.get('/api/decisions', async (request, reply) => {
  const result = await pool.query(
    `SELECT tx_hash, sender, recipient, decision, risk_score, reason_codes, created_at
     FROM compliance_decisions
     ORDER BY created_at DESC
     LIMIT 50`
  );
  return result.rows;
});

// REST: recent blocks (post-execution proposer attribution)
fastify.get('/api/blocks', async (request, reply) => {
  const result = await pool.query(
    `SELECT block_hash, block_number, builder_address, compliance_status, tx_count, created_at
     FROM blocks
     ORDER BY block_number DESC
     LIMIT 50`
  );
  return result.rows;
});

// Simple summary stats for dashboard header
fastify.get('/api/stats', async (request, reply) => {
  const decisions = await pool.query(
    `SELECT decision, COUNT(*) FROM compliance_decisions GROUP BY decision`
  );
  const blocks = await pool.query(
    `SELECT compliance_status, COUNT(*) FROM blocks GROUP BY compliance_status`
  );
  return { decisions: decisions.rows, blocks: blocks.rows };
});

// Poll Postgres periodically and broadcast new rows to connected dashboards
let lastDecisionCount = 0;
setInterval(async () => {
  const result = await pool.query(
    `SELECT tx_hash, decision, risk_score, reason_codes, created_at
     FROM compliance_decisions
     ORDER BY created_at DESC
     LIMIT 1`
  );
  if (result.rows.length > 0) {
    const countResult = await pool.query(`SELECT COUNT(*) FROM compliance_decisions`);
    const currentCount = parseInt(countResult.rows[0].count, 10);
    if (currentCount !== lastDecisionCount) {
      lastDecisionCount = currentCount;
      broadcast({ type: 'new_decision', data: result.rows[0] });
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
