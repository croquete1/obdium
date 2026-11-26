// Graph samples must represent observed measurements, not fabricated zeros.
export function createGraphSampler() {
  let pending = null;
  return {
    push(payload) {
      const unit = typeof payload?.unit === "string"
        ? payload.unit.trim().toUpperCase()
        : null;
      pending = unit !== null && unit !== "NO DATA" && Number.isFinite(payload?.value)
        ? { value: payload.value, unit }
        : null;
    },
    take() {
      const sample = pending;
      pending = null;
      return sample;
    },
  };
}
