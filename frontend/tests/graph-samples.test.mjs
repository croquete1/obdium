import assert from "node:assert/strict";
import { test } from "node:test";
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
