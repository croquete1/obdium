import assert from "node:assert/strict";
import { test } from "node:test";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { createTrackerRegistry } from "../scripts/connection-lifecycle.mjs";
import { createGraphSampler } from "../scripts/graph-samples.mjs";

test("an actual zero is a valid sample", () => {
  const sampler = createGraphSampler();
  sampler.push({ value: 0, unit: "rpm" });
  assert.deepEqual(sampler.take(), { value: 0, unit: "RPM" });
});

test("missing and nonfinite measurements produce gaps", () => {
  const sampler = createGraphSampler();
  for (const payload of [
    { value: 0, unit: "NO DATA" },
    { value: 42, unit: " no data " },
    { value: null, unit: "RPM" },
    { value: NaN, unit: "RPM" },
    { value: Infinity, unit: "RPM" },
    { value: "0", unit: "RPM" },
    { value: 0 },
    undefined,
  ]) {
    sampler.push(payload);
    assert.equal(sampler.take(), null);
  }
});

test("each observed sample is consumed at most once", () => {
  const sampler = createGraphSampler();
  assert.equal(sampler.take(), null);
  sampler.push({ value: 1200, unit: "RPM" });
  assert.equal(sampler.take().value, 1200);
  assert.equal(sampler.take(), null);
  assert.equal(sampler.take(), null);
});

test("an unavailable update invalidates an earlier pending measurement", () => {
  const sampler = createGraphSampler();
  sampler.push({ value: 1200, unit: "RPM" });
  sampler.push({ value: 0, unit: "NO DATA" });
  assert.equal(sampler.take(), null);
});

test("the latest valid observation replaces earlier pending observations", () => {
  const sampler = createGraphSampler();
  sampler.push({ value: 0, unit: "NO DATA" });
  sampler.push({ value: 10, unit: "km/h" });
  sampler.push({ value: 6, unit: "mph" });
  assert.deepEqual(sampler.take(), { value: 6, unit: "MPH" });
  assert.equal(sampler.take(), null);
});

test("dimensionless and negative measurements are preserved", () => {
  const sampler = createGraphSampler();
  sampler.push({ value: -2, unit: "" });
  assert.deepEqual(sampler.take(), { value: -2, unit: "" });
});

test("actual graph listener ignores malformed identities and uses monotonic elapsed time", async () => {
  const source = readFileSync(new URL("../scripts/features.js", import.meta.url), "utf8");
  const start = source.indexOf("const activeTrackers = createTrackerRegistry();");
  const end = source.indexOf("export function appendTerminalOutput", start);
  assert.ok(start >= 0 && end > start);
  const graph = {
    data: { labels: [], datasets: [{ data: [] }] },
    options: { scales: { x: { title: {} }, y: { title: { text: "RPM" } } } },
    config: { options: {} },
    update() {},
  };
  let monotonic = 100;
  let listener;
  let tick;
  let released = 0;
  const context = {
    createGraphSampler: () => createGraphSampler(() => monotonic),
    createTrackerRegistry,
    window: { connected: true },
    Chart: { getChart: () => graph },
    performance: { now: () => monotonic },
    Date: { now: () => { throw new Error("wall clock must not be used"); } },
    listen: async (name, callback) => {
      assert.equal(name, "update-graphs");
      listener = callback;
      return () => released++;
    },
    setInterval(callback) { tick = callback; return 1; },
    clearInterval() { released++; },
  };
  runInNewContext(
    source.slice(start, end).replaceAll("export ", "") +
      "\nthis.trackGraph = trackGraph; this.stopGraphTracking = stopGraphTracking;",
    context,
  );
  await context.trackGraph("rpm", "RPM", "RPM");
  listener({ payload: { name: " rpm ", value: 0, unit: "rpm" } });
  for (const event of [undefined, {}, { payload: null }, { payload: { name: 7 } },
    { payload: { name: {} } }, { payload: { name: "speed", value: 42, unit: "mph" } }]) {
    listener(event);
  }
  monotonic = 1100;
  tick();
  assert.deepEqual(Array.from(graph.data.labels), ["0:01"]);
  assert.deepEqual(Array.from(graph.data.datasets[0].data), [0]);
  listener({ payload: { name: "RPM", value: 0, unit: "NO DATA" } });
  monotonic = 2100;
  tick();
  assert.deepEqual(Array.from(graph.data.labels), ["0:01", "0:02"]);
  assert.deepEqual(Array.from(graph.data.datasets[0].data), [0, null]);
  // A delayed timer must not present an old queued reading as current.
  listener({ payload: { name: "RPM", value: 1200, unit: "RPM" } });
  monotonic = 4100;
  tick();
  assert.deepEqual(Array.from(graph.data.labels), ["0:01", "0:02", "0:04"]);
  assert.deepEqual(Array.from(graph.data.datasets[0].data), [0, null, null]);
  context.stopGraphTracking();
  assert.equal(released, 2);
  listener({ payload: { name: "RPM", value: 5000, unit: "RPM" } });
  monotonic = 5100;
  tick();
  assert.deepEqual(Array.from(graph.data.datasets[0].data), [0, null, null]);
});

test("queued measurement expires at the exclusive age boundary", () => {
  let now = 100;
  const sampler = createGraphSampler(() => now);
  sampler.push({ value: 0, unit: "RPM" });
  now = 2099;
  assert.deepEqual(sampler.take(), { value: 0, unit: "RPM" });
  sampler.push({ value: 1200, unit: "RPM" });
  now = 4099;
  assert.equal(sampler.take(), null);
  assert.equal(sampler.take(), null);
  sampler.push({ value: 800, unit: "RPM" });
  now = 4100;
  assert.deepEqual(sampler.take(), { value: 800, unit: "RPM" });
});

test("invalid or reversed receipt clock cannot publish a measurement", () => {
  let now = 100;
  const sampler = createGraphSampler(() => now);
  sampler.push({ value: 1200, unit: "RPM" });
  now = 99;
  assert.equal(sampler.take(), null);
  now = NaN;
  sampler.push({ value: 1200, unit: "RPM" });
  now = 100;
  assert.equal(sampler.take(), null);
  sampler.push({ value: 1200, unit: "RPM" });
  now = Infinity;
  assert.equal(sampler.take(), null);
});
