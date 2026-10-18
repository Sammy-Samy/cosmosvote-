import { createServer, Server } from 'http';
import { RateLimiter, applyRateLimit } from './rateLimiter';

export function createHealthServer(
  checkReadiness: () => Promise<boolean>,
  getUptime: () => number = () => process.uptime()
): Server {
  const limiter = new RateLimiter();
  // Clean up stale entries every 5 minutes
  limiter.startCleanup();

  return createServer(async (request, response) => {
    // Apply per-IP rate limiting to all routes
    if (applyRateLimit(limiter, request, response)) return;

    response.setHeader('Content-Type', 'application/json');

    if (request.method === 'GET' && request.url === '/health') {
      response.writeHead(200);
      response.end(JSON.stringify({ status: 'ok', uptime: getUptime() }));
      return;
    }

    if (request.method === 'GET' && request.url === '/ready') {
      const ready = await checkReadiness().catch(() => false);
      response.writeHead(ready ? 200 : 503);
      response.end(JSON.stringify({ status: ready ? 'ready' : 'not_ready' }));
      return;
    }

    response.writeHead(404);
    response.end(JSON.stringify({ status: 'not_found' }));
  });
}
