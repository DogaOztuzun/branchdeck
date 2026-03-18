import { query } from "@anthropic-ai/claude-agent-sdk";
import { readFileSync } from "node:fs";
import { createInterface } from "node:readline";

/** @type {AbortController | null} */
let activeAbort = null;

/** @type {string | null} */
let activeSessionId = null;

/**
 * Write a JSON message to stdout (newline-delimited).
 * @param {Record<string, unknown>} msg
 */
function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}

/**
 * Handle a launch_run request from the Rust backend.
 * @param {object} request
 * @param {string} request.task_path
 * @param {string} request.worktree
 * @param {object} [request.options]
 * @param {number} [request.options.max_turns]
 * @param {number} [request.options.max_budget_usd]
 */
async function handleLaunchRun(request) {
  if (activeAbort) {
    send({
      type: "run_error",
      status: "failed",
      error: "A run is already active",
      session_id: activeSessionId,
    });
    return;
  }

  let taskContent;
  try {
    taskContent = readFileSync(request.task_path, "utf-8");
  } catch (err) {
    send({
      type: "run_error",
      status: "failed",
      error: `Failed to read task file: ${err.message}`,
      session_id: null,
    });
    return;
  }

  activeAbort = new AbortController();

  const queryOptions = {
    prompt: taskContent,
    options: {
      cwd: request.worktree,
      abortController: activeAbort,
      permissionMode: "acceptEdits",
    },
  };

  if (request.options?.max_turns) {
    queryOptions.options.maxTurns = request.options.max_turns;
  }
  if (request.options?.max_budget_usd) {
    queryOptions.options.maxBudgetUsd = request.options.max_budget_usd;
  }

  try {
    const conversation = query(queryOptions);

    for await (const message of conversation) {
      if (!message || !message.type) {
        continue;
      }

      switch (message.type) {
        case "system": {
          // SystemMessage (init) - session started
          if (message.session_id) {
            activeSessionId = message.session_id;
            send({
              type: "session_started",
              session_id: message.session_id,
            });
          }
          break;
        }

        case "assistant": {
          // AssistantMessage - extract text content
          if (message.content) {
            for (const block of message.content) {
              if (block.type === "text" && block.text) {
                send({
                  type: "assistant_text",
                  text: block.text,
                  session_id: activeSessionId,
                });
              }
              if (block.type === "tool_use") {
                send({
                  type: "tool_call",
                  tool: block.name,
                  file_path: block.input?.file_path ?? null,
                  session_id: activeSessionId,
                });

                // Map file-related tools to run_step
                if (
                  ["Edit", "Write", "MultiEdit"].includes(block.name) &&
                  block.input?.file_path
                ) {
                  send({
                    type: "run_step",
                    step: "files_changed",
                    detail: block.input.file_path,
                    session_id: activeSessionId,
                  });
                }
              }
            }
          }
          break;
        }

        case "result": {
          // ResultMessage - run complete or error
          const costUsd = message.total_cost_usd ?? 0;

          if (
            message.subtype === "success" ||
            message.subtype === "end_turn"
          ) {
            send({
              type: "run_complete",
              status: "succeeded",
              cost_usd: costUsd,
              session_id: activeSessionId,
            });
          } else {
            send({
              type: "run_error",
              status: "failed",
              error: message.subtype ?? "unknown error",
              cost_usd: costUsd,
              session_id: activeSessionId,
            });
          }

          activeAbort = null;
          activeSessionId = null;
          break;
        }

        default: {
          // Log unhandled message types to stderr for debugging
          console.error(
            `Unhandled message type: ${message.type}`,
            JSON.stringify(message).slice(0, 200),
          );
          break;
        }
      }
    }
  } catch (err) {
    const errorMsg =
      err.name === "AbortError" ? "Run cancelled" : err.message;
    const status = err.name === "AbortError" ? "cancelled" : "failed";

    send({
      type: "run_error",
      status,
      error: errorMsg,
      session_id: activeSessionId,
    });

    activeAbort = null;
    activeSessionId = null;
  }
}

/**
 * Handle a cancel_run request from the Rust backend.
 */
function handleCancelRun() {
  if (activeAbort) {
    console.error("Cancelling active run");
    activeAbort.abort();
  } else {
    send({
      type: "run_error",
      status: "failed",
      error: "No active run to cancel",
      session_id: null,
    });
  }
}

// --- Main stdin reader ---

const rl = createInterface({
  input: process.stdin,
  terminal: false,
});

rl.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) {
    return;
  }

  let request;
  try {
    request = JSON.parse(trimmed);
  } catch (err) {
    console.error(`Failed to parse stdin JSON: ${err.message}`);
    return;
  }

  switch (request.type) {
    case "launch_run":
      handleLaunchRun(request).catch((err) => {
        console.error(`Unhandled error in launch_run: ${err.message}`);
        send({
          type: "run_error",
          status: "failed",
          error: `Internal bridge error: ${err.message}`,
          session_id: activeSessionId,
        });
        activeAbort = null;
        activeSessionId = null;
      });
      break;

    case "cancel_run":
      handleCancelRun();
      break;

    default:
      console.error(`Unknown request type: ${request.type}`);
      break;
  }
});

// Graceful shutdown on stdin close
rl.on("close", () => {
  console.error("stdin closed, shutting down sidecar");
  if (activeAbort) {
    activeAbort.abort();
  }
  process.exit(0);
});

// Handle SIGTERM/SIGINT
for (const signal of ["SIGTERM", "SIGINT"]) {
  process.on(signal, () => {
    console.error(`Received ${signal}, shutting down`);
    if (activeAbort) {
      activeAbort.abort();
    }
    process.exit(0);
  });
}

console.error("Branchdeck agent bridge started");
