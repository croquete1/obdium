import assert from "node:assert/strict";
import { test } from "node:test";
import { readFileSync } from "node:fs";
import { normalizeReadinessTest, readinessLabels, replaceReadinessSnapshot, getReadinessSnapshot } from "../scripts/readiness-state.mjs";

test("supported completed and incomplete monitors have distinct states", () => {
  for (const complete of [true, false]) {
    const data = normalizeReadinessTest({ name: "Catalyst", available: true, complete });
    assert.deepEqual(data, { name: "Catalyst", available: true, complete, status: complete ? "complete" : "incomplete" });
    assert.deepEqual(readinessLabels(data), { availability: "SUPPORTED", completeness: complete ? "READY" : "NOT READY" });
  }
});

test("unsupported monitor never claims completion even if backend bit says complete", () => {
  const data = normalizeReadinessTest({ name: "Catalyst", available: false, complete: true });
  assert.deepEqual(data, { name: "Catalyst", available: false, complete: null, status: "unsupported" });
  assert.deepEqual(readinessLabels(data), { availability: "UNSUPPORTED", completeness: "NOT APPLICABLE" });
});

test("legacy Unknown and reserved monitors do not claim availability or completion", () => {
  for (const name of ["Unknown", "Reserved"]) {
    const data = normalizeReadinessTest({ name, available: true, complete: true });
    assert.deepEqual(data, { name, available: null, complete: null, status: "unknown" });
    assert.deepEqual(readinessLabels(data), { availability: "UNKNOWN", completeness: "UNKNOWN" });
  }
});

test("malformed or missing flags cannot become ready", () => {
  for (const input of [null, {}, { name: "Catalyst", available: "true", complete: true }]) {
    assert.equal(normalizeReadinessTest(input).status, "unknown");
  }
  const data = normalizeReadinessTest({ name: "Catalyst", available: true, complete: "false" });
  assert.equal(data.complete, null);
  assert.equal(readinessLabels(data).completeness, "UNKNOWN");
});

test("JSON snapshot contains data rather than localized presentation text", () => {
  replaceReadinessSnapshot([{ name: "Catalyst", available: true, complete: false }]);
  const result = getReadinessSnapshot();
  assert.deepEqual(JSON.parse(JSON.stringify(result)), [
    { name: "Catalyst", available: true, complete: false, status: "incomplete" },
  ]);
  result[0].complete = true;
  assert.equal(getReadinessSnapshot()[0].complete, false);
});

test("clearing or replacing snapshot discards previous observations", () => {
  replaceReadinessSnapshot([{ name: "Catalyst", available: true, complete: true }]);
  assert.deepEqual(replaceReadinessSnapshot(null), []);
  assert.deepEqual(getReadinessSnapshot(), []);
  replaceReadinessSnapshot([{ name: "Misfire", available: false, complete: false }]);
  assert.deepEqual(replaceReadinessSnapshot([]), []);
});

test("production export uses the shared snapshot, not DOM labels, and disconnect clears it", () => {
  const main = readFileSync(new URL("../scripts/main.js", import.meta.url), "utf8");
  const events = readFileSync(new URL("../scripts/events.js", import.meta.url), "utf8");
  const start = main.indexOf('imTestExportButton.addEventListener');
  const end = main.indexOf('const vinExportButton', start);
  assert.ok(start >= 0 && end > start);
  const block = main.slice(start, end);
  assert.match(block, /getReadinessSnapshot\(\)/);
  assert.match(block, /!window\.connected \|\| totalJSON\.length === 0/);
  assert.doesNotMatch(block, /test-availability|test-completeness|\.textContent/);
  assert.match(events, /window\.connected = false;\s*replaceReadinessSnapshot\(\[\]\);/);
  assert.match(events, /listen\("update-readiness-tests", \(event\) => \{\s*if \(!window\.connected\) return;/);
});
