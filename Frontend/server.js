import { createServer } from 'http';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PORT = process.env.PORT || 3001;
const HOST = process.env.HOST || '0.0.0.0';

// Simple static file server with health check endpoint
const server = createServer((req, res) => {
  // Health check endpoint for Railway
  if (req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok', timestamp: new Date().toISOString() }));
    return;
  }

  // Serve static files from the dist directory
  let filePath = join(__dirname, 'dist', req.url === '/' ? 'index.html' : req.url);

  try {
    // For client-side routing, serve index.html for non-file requests
    if (!filePath.includes('.')) {
      filePath = join(__dirname, 'dist', 'index.html');
    }

    const content = readFileSync(filePath);

    // Set content type based on file extension
    const extname = String(filePath.split('.').pop()).toLowerCase();
    const mimeTypes = {
      html: 'text/html',
      js: 'text/javascript',
      css: 'text/css',
      json: 'application/json',
      png: 'image/png',
      jpg: 'image/jpg',
      gif: 'image/gif',
      svg: 'image/svg+xml',
    };

    const contentType = mimeTypes[extname] || 'application/octet-stream';
    res.writeHead(200, { 'Content-Type': contentType });
    res.end(content, 'utf-8');
  } catch (error) {
    if (error.code === 'ENOENT') {
      // If file not found, serve index.html for client-side routing
      try {
        const content = readFileSync(join(__dirname, 'dist', 'index.html'));
        res.writeHead(200, { 'Content-Type': 'text/html' });
        res.end(content, 'utf-8');
      } catch (e) {
        res.writeHead(404);
        res.end('Not found');
      }
    } else {
      // Server error
      res.writeHead(500);
      res.end(`Server Error: ${error.code}`);
    }
  }
});

server.listen(PORT, HOST, () => {
  console.log(`Server running at http://${HOST}:${PORT}/`);
  console.log(`Health check available at http://${HOST}:${PORT}/health`);
});import { createServer } from 'http';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PORT = process.env.PORT || 3001;
const HOST = process.env.HOST || '0.0.0.0';

// Simple static file server with health check endpoint
const server = createServer((req, res) => {
  // Health check endpoint for Railway
  if (req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok', timestamp: new Date().toISOString() }));
    return;
  }

  // Serve static files from the dist directory
  let filePath = join(__dirname, 'dist', req.url === '/' ? 'index.html' : req.url);

  try {
    // For client-side routing, serve index.html for non-file requests
    if (!filePath.includes('.')) {
      filePath = join(__dirname, 'dist', 'index.html');
    }

    const content = readFileSync(filePath);

    // Set content type based on file extension
    const extname = String(filePath.split('.').pop()).toLowerCase();
    const mimeTypes = {
      html: 'text/html',
      js: 'text/javascript',
      css: 'text/css',
      json: 'application/json',
      png: 'image/png',
      jpg: 'image/jpg',
      gif: 'image/gif',
      svg: 'image/svg+xml',
    };

    const contentType = mimeTypes[extname] || 'application/octet-stream';
    res.writeHead(200, { 'Content-Type': contentType });
    res.end(content, 'utf-8');
  } catch (error) {
    if (error.code === 'ENOENT') {
      // If file not found, serve index.html for client-side routing
      try {
        const content = readFileSync(join(__dirname, 'dist', 'index.html'));
        res.writeHead(200, { 'Content-Type': 'text/html' });
        res.end(content, 'utf-8');
      } catch (e) {
        res.writeHead(404);
        res.end('Not found');
      }
    } else {
      // Server error
      res.writeHead(500);
      res.end(`Server Error: ${error.code}`);
    }
  }
});

server.listen(PORT, HOST, () => {
  console.log(`Server running at http://${HOST}:${PORT}/`);
  console.log(`Health check available at http://${HOST}:${PORT}/health`);
});
