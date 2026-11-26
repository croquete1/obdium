import assert from "node:assert/strict";
import { test } from "node:test";
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
