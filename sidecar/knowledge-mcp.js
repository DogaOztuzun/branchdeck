#!/usr/bin/env node
// MCP server for Branchdeck Knowledge Service.
// Exposes query_knowledge and remember_this tools via stdio transport.
// Proxies requests to Branchdeck's TCP endpoint.

const http = require("http");
const readline = require("readline");

const KNOWLEDGE_PORT = process.env.BRANCHDECK_KNOWLEDGE_PORT;
if (!KNOWLEDGE_PORT) {
  process.stderr.write(
    "BRANCHDECK_KNOWLEDGE_PORT not set, cannot connect to knowledge service\n",
  );
  process.exit(1);
}

// --- HTTP helper to call Branchdeck TCP endpoint ---

function postJSON(path, body) {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify(body);
    const req = http.request(
      {
        hostname: "127.0.0.1",
        port: parseInt(KNOWLEDGE_PORT, 10),
        path,
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Content-Length": Buffer.byteLength(data),
        },
        timeout: 30000,
      },
      (res) => {
        let chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("error", (e) => reject(e));
        res.on("end", () => {
          try {
            resolve(JSON.parse(Buffer.concat(chunks).toString()));
          } catch {
            resolve({ error: "invalid response from knowledge service" });
          }
        });
      },
    );
    req.on("error", (e) => reject(e));
    req.on("timeout", () => {
      req.destroy();
      reject(new Error("timeout"));
    });
    req.write(data);
    req.end();
  });
}

// --- MCP Protocol (JSON-RPC over stdio) ---

const TOOLS = [
  {
    name: "query_knowledge",
    description:
      "Search Branchdeck's knowledge store for relevant past experiences, fixes, and patterns. Returns semantically similar entries from trajectories and explicit memories.",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "Natural language search query",
        },
        top_k: {
          type: "number",
          description: "Number of results to return (default 5, max 100)",
        },
      },
      required: ["query"],
    },
  },
  {
    name: "remember_this",
    description:
      "Save important knowledge to Branchdeck's persistent memory. Use when you learn something worth remembering — a fix, pattern, convention, or insight that could help in future sessions.",
    inputSchema: {
      type: "object",
      properties: {
        content: {
          type: "string",
          description: "The knowledge to remember",
        },
      },
      required: ["content"],
    },
  },
  {
    name: "suggest_next",
    description:
      "Get workflow suggestions based on learned patterns from past sessions. Describe what you're currently doing and get pattern-based recommendations.",
    inputSchema: {
      type: "object",
      properties: {
        context: {
          type: "string",
          description:
            "Description of current context — what you're working on, what you just did",
        },
        top_k: {
          type: "number",
          description: "Number of suggestions to return (default 5, max 20)",
        },
      },
      required: ["context"],
    },
  },
];

const SERVER_INFO = {
  name: "branchdeck-knowledge",
  version: "0.1.0",
};

const CAPABILITIES = {
  tools: {},
};

function makeResponse(id, result) {
  return { jsonrpc: "2.0", id, result };
}

function makeError(id, code, message) {
  return { jsonrpc: "2.0", id, error: { code, message } };
}

async function handleRequest(msg) {
  const { id, method, params } = msg;

  switch (method) {
    case "initialize":
      return makeResponse(id, {
        protocolVersion: "2024-11-05",
        serverInfo: SERVER_INFO,
        capabilities: CAPABILITIES,
      });

    case "notifications/initialized":
      return null; // notification, no response

    case "tools/list":
      return makeResponse(id, { tools: TOOLS });

    case "tools/call": {
      const toolName = params?.name;
      const args = params?.arguments || {};

      try {
        if (toolName === "query_knowledge") {
          const query = (args.query || "").trim();
          if (!query) {
            return makeResponse(id, {
              content: [
                { type: "text", text: "Error: query must not be empty" },
              ],
              isError: true,
            });
          }

          const results = await postJSON("/knowledge/query", {
            query,
            top_k: args.top_k ?? 5,
          });

          if (results.error) {
            return makeResponse(id, {
              content: [{ type: "text", text: `Error: ${results.error}` }],
              isError: true,
            });
          }

          const entries = Array.isArray(results) ? results : [];
          const text =
            entries.length === 0
              ? "No matching knowledge found."
              : entries
                  .map(
                    (e, i) =>
                      `[${i + 1}] (distance: ${e.distance?.toFixed(3) || "?"}) ${e.content}`,
                  )
                  .join("\n\n");

          return makeResponse(id, {
            content: [{ type: "text", text }],
          });
        }

        if (toolName === "remember_this") {
          const content = (args.content || "").trim();
          if (!content) {
            return makeResponse(id, {
              content: [
                { type: "text", text: "Error: content must not be empty" },
              ],
              isError: true,
            });
          }

          const result = await postJSON("/knowledge/remember", {
            content,
          });

          if (result.error) {
            return makeResponse(id, {
              content: [{ type: "text", text: `Error: ${result.error}` }],
              isError: true,
            });
          }

          const idVal = result.id ?? "queued";
          return makeResponse(id, {
            content: [
              {
                type: "text",
                text: `Remembered (id: ${idVal}). This knowledge is now stored and searchable in future sessions.`,
              },
            ],
          });
        }

        if (toolName === "suggest_next") {
          const context = (args.context || "").trim();
          if (!context) {
            return makeResponse(id, {
              content: [
                { type: "text", text: "Error: context must not be empty" },
              ],
              isError: true,
            });
          }

          const results = await postJSON("/knowledge/suggest", {
            context,
            top_k: args.top_k ?? 5,
          });

          if (results.error) {
            return makeResponse(id, {
              content: [{ type: "text", text: `Error: ${results.error}` }],
              isError: true,
            });
          }

          const entries = Array.isArray(results) ? results : [];
          const text =
            entries.length === 0
              ? "No learned patterns yet. Keep working — suggestions emerge after enough sessions."
              : entries
                  .map(
                    (e, i) =>
                      `[${i + 1}] (quality: ${e.avgQuality?.toFixed(2) || "?"}) ${e.content}`,
                  )
                  .join("\n\n");

          return makeResponse(id, { content: [{ type: "text", text }] });
        }

        return makeError(id, -32601, `Unknown tool: ${toolName}`);
      } catch (e) {
        return makeResponse(id, {
          content: [
            {
              type: "text",
              text: `Knowledge service unavailable: ${e.message}`,
            },
          ],
          isError: true,
        });
      }
    }

    default:
      return makeError(id, -32601, `Method not found: ${method}`);
  }
}

// --- Stdio transport ---

const rl = readline.createInterface({ input: process.stdin });

rl.on("line", async (line) => {
  if (!line.trim()) return;
  try {
    const msg = JSON.parse(line);
    const response = await handleRequest(msg);
    if (response) {
      process.stdout.write(JSON.stringify(response) + "\n");
    }
  } catch (e) {
    process.stderr.write(`MCP parse error: ${e.message}\n`);
  }
});

rl.on("close", () => process.exit(0));

process.stderr.write(
  `branchdeck-knowledge MCP server started (port: ${KNOWLEDGE_PORT})\n`,
);
