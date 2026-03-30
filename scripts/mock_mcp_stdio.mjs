import readline from "node:readline";

function write(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity
});

for await (const line of rl) {
  const trimmed = line.trim();
  if (!trimmed) {
    continue;
  }

  let request;
  try {
    request = JSON.parse(trimmed);
  } catch {
    continue;
  }

  const { id, method, params } = request;
  if (method === "notifications/initialized") {
    continue;
  }

  if (method === "initialize") {
    write({
      jsonrpc: "2.0",
      id,
      result: {
        protocolVersion: "2024-11-05",
        capabilities: { tools: {} },
        serverInfo: { name: "zeroclawx-mock", version: "1.0.0" }
      }
    });
    continue;
  }

  if (method === "tools/list") {
    write({
      jsonrpc: "2.0",
      id,
      result: {
        tools: [
          {
            name: "echo_weather",
            description: "Return a deterministic weather report for a city.",
            inputSchema: {
              type: "object",
              properties: {
                city: { type: "string" }
              },
              required: ["city"]
            }
          }
        ]
      }
    });
    continue;
  }

  if (method === "tools/call") {
    const toolName = params?.name;
    const city = params?.arguments?.city ?? "Unknown";
    if (toolName === "echo_weather") {
      write({
        jsonrpc: "2.0",
        id,
        result: {
          content: [
            {
              type: "text",
              text: `Weather for ${city}: clear skies from the MCP mock server.`
            }
          ]
        }
      });
      continue;
    }
  }

  write({
    jsonrpc: "2.0",
    id,
    error: {
      code: -32601,
      message: `Unsupported method: ${method}`
    }
  });
}
