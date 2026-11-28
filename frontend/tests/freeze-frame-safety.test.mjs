import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const html = read("../index.html");
const settings = read("../scripts/settings.js");
const features = read("../scripts/features.js");
const bridge = read("../../backend/src/bridge/events.rs");

test("freeze-frame control is disabled and explains why", () => {
  const control = html.match(/<input\b[^>]*\bid="use-freeze-frame"[^>]*>/s);
  assert.ok(control);
  assert.match(control[0], /\bdisabled\b/);
  assert.match(control[0], /aria-describedby="freeze-frame-unavailable"/);
  assert.match(html, /id="freeze-frame-unavailable"[\s\S]*?temporarily unavailable/);
});

test("frontend cannot optimistically claim freeze-frame mode is active", () => {
  assert.doesNotMatch(html, /FREEZE FRAME ACTIVE|id="freeze-frame-status"/);
  assert.doesNotMatch(settings + features, /\bfreezeFrameDisclaimer\b|\bfreezeFrameBar\b/);
  assert.doesNotMatch(settings, /["']use-freeze-frame["']/);
});

test("neither spelling of the old IPC setting activates implicit mode 02", () => {
  assert.doesNotMatch(bridge, /["']use-freeze-fram(?:e)?["']/);
  assert.doesNotMatch(bridge, /\.query_freeze_frame\s*\(/);
});

test("normal connection, scan and DTC export remain available", () => {
  assert.match(html, /id="btn-connect"/);
  assert.match(html, /id="dtc-scan-button"/);
  assert.match(html, /id="dtc-log-file"/);
  assert.match(bridge, /window\.listen\("get-dtcs"/);
});
