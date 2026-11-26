import assert from "node:assert/strict";
import { test } from "node:test";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { createTrackerRegistry, runConnectionQueries } from "../scripts/connection-lifecycle.mjs";

test("a late listener registration is disposed and cannot replace its successor", () => {
  const registry = createTrackerRegistry();
  const first = registry.begin("rpm");
  const second = registry.begin("rpm");
  let released = 0;
  assert.equal(first.attach(() => released++), false);
  assert.equal(released, 1);
  assert.equal(first.isCurrent(), false);
  assert.equal(second.isCurrent(), true);
});

test("replacing a tracker disposes its timer and listener exactly once", () => {
  const registry = createTrackerRegistry();
  const calls = [];
  const first = registry.begin("rpm");
  first.attach(() => calls.push("listener"));
  first.attach(() => calls.push("timer"));
  const second = registry.begin("rpm");
  first.stop();
  assert.deepEqual(calls, ["timer", "listener"]);
  assert.equal(second.isCurrent(), true);
});

test("disconnect invalidates all pending registrations", () => {
  const registry = createTrackerRegistry();
  const first = registry.begin("rpm");
  const second = registry.begin("speed");
  let released = 0;
  second.attach(() => released++);
  registry.stopAll();
  registry.stopAll();
  assert.equal(first.attach(() => released++), false);
  assert.equal(second.isCurrent(), false);
  assert.equal(released, 2);
});

test("cleanup errors do not prevent other resources from being released", () => {
  const errors = [];
  const registry = createTrackerRegistry((error) => errors.push(error.message));
  const ticket = registry.begin("rpm");
  let released = false;
  ticket.attach(() => { released = true; });
  ticket.attach(() => { throw new Error("cleanup failed"); });
  registry.stopAll();
  assert.equal(released, true);
  assert.deepEqual(errors, ["cleanup failed"]);
});

test("one hundred reconnect cycles release every registered resource", () => {
  const registry = createTrackerRegistry();
  let released = 0;
  for (let i = 0; i < 100; i++) {
    const ticket = registry.begin("rpm");
    ticket.attach(() => released++);
    ticket.attach(() => released++);
    registry.stopAll();
    assert.equal(ticket.isCurrent(), false);
  }
  assert.equal(released, 200);
});

test("connection invalidation before first delay blocks both queries", async () => {
  const events = [];
  let current = true;
  await runConnectionQueries({
    isCurrent: () => current,
    delay: async () => { current = false; },
    emit: (event) => events.push(event),
    firstDelay: 7000,
  });
  assert.deepEqual(events, []);
});

test("connection invalidation between queries blocks the second query", async () => {
  const events = [];
  let current = true;
  await runConnectionQueries({
    isCurrent: () => current,
    delay: async (ms) => { if (ms === 2000) current = false; },
    emit: (event) => events.push(event),
    firstDelay: 15000,
  });
  assert.deepEqual(events, ["get-pids"]);
});

test("current connection dispatches both queries after original delays", async () => {
  const events = [];
  const delays = [];
  await runConnectionQueries({
    isCurrent: () => true,
    delay: async (ms) => { delays.push(ms); },
    emit: (event) => events.push(event),
    firstDelay: 7000,
  });
  assert.deepEqual(delays, [7000, 2000]);
  assert.deepEqual(events, ["get-pids", "get-readiness-tests"]);
});

test("failed first dispatch propagates and does not send readiness request", async () => {
  const events = [];
  await assert.rejects(runConnectionQueries({
    isCurrent: () => true,
    delay: async () => {},
    emit: async (event) => {
      events.push(event);
      throw new Error("IPC failed");
    },
    firstDelay: 7000,
  }), /IPC failed/);
  assert.deepEqual(events, ["get-pids"]);
});

test("invalidation during in-flight dispatch prevents follow-up delay and request", async () => {
  const events = [];
  const delays = [];
  let current = true;
  await runConnectionQueries({
    isCurrent: () => current,
    delay: async (ms) => { delays.push(ms); },
    emit: async (event) => {
      events.push(event);
      await Promise.resolve();
      current = false;
    },
    firstDelay: 7000,
  });
  assert.deepEqual(events, ["get-pids"]);
  assert.deepEqual(delays, [7000]);
});

test("failure of readiness dispatch is observed by caller", async () => {
  const events = [];
  await assert.rejects(runConnectionQueries({
    isCurrent: () => true,
    delay: async () => {},
    emit: async (event) => {
      events.push(event);
      if (event === "get-readiness-tests") throw new Error("readiness IPC failed");
    },
    firstDelay: 7000,
  }), /readiness IPC failed/);
  assert.deepEqual(events, ["get-pids", "get-readiness-tests"]);
});

test("actual status handler re-enables connection controls after failed connection", async () => {
  const source = readFileSync(new URL("../scripts/events.js", import.meta.url), "utf8");
  const start = source.indexOf("let connectionGeneration = 0;");
  const end = source.indexOf('listen("update-pids",', start);
  assert.ok(start >= 0 && end > start);
  for (const connected of [false, true]) {
    const notices = [];
    const connectButton = { disabled: true };
    const disconnectButton = { disabled: !connected };
    const recordResponses = { disabled: false };
    const replayResponses = { disabled: false };
    const demoStatus = { style: { display: "flex" } };
    const connectionLabel = { textContent: "" };
    const connectionIcon = { src: "" };
    const status = { textContent: "" };
    const window = { connected };
    let stopped = 0;
    let handler;
    runInNewContext(source.slice(start, end), {
      window, connectButton, disconnectButton, recordResponses, replayResponses,
      demoStatus, connectionLabel, connectionIcon,
      document: { getElementById: () => ({ ...status, replaceChildren() {} }) },
      replaceReadinessSnapshot() {},
      stopGraphTracking() { stopped++; },
      addNotification: (...args) => notices.push(args),
      listen(name, callback) {
        assert.equal(name, "connection-status");
        handler = callback;
      },
    });
    assert.equal(typeof handler, "function");
    await handler({ payload: { connected: false, message: "Connection ended" } });
    assert.equal(window.connected, false);
    assert.equal(connectButton.disabled, false);
    assert.equal(disconnectButton.disabled, true);
    assert.equal(recordResponses.disabled, true);
    assert.equal(replayResponses.disabled, true);
    assert.equal(demoStatus.style.display, "none");
    assert.equal(stopped, 1);
    assert.deepEqual(notices, [[connected ? "DISCONNECTED FROM ELM" : "CONNECTION NOT ESTABLISHED", "Connection ended"]]);
  }
});
