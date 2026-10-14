/** Cumulative donation thresholds, from `codes::JUNK_TIERS`. */
export const JUNK_TIERS: readonly { tier: number; threshold: number }[] = [
  { tier: 0, threshold: 0 },
  { tier: 1, threshold: 1_000 },
  { tier: 2, threshold: 6_000 },
  { tier: 3, threshold: 26_000 },
  { tier: 4, threshold: 86_000 },
  { tier: 5, threshold: 206_000 },
  { tier: 6, threshold: 456_000 },
  { tier: 7, threshold: 956_000 },
  { tier: 8, threshold: 1_956_000 },
  { tier: 9, threshold: 3_956_000 },
];

/** The highest junk-shop tier whose threshold `counter` has reached. */
export function junkTierFromCounter(counter: number): number {
  let best = 0;
  for (const { tier, threshold } of JUNK_TIERS) {
    if (counter >= threshold) best = tier;
  }
  return best;
}

/** The counter value that selects `tier`, if the tier exists. */
export function junkThreshold(tier: number): number | undefined {
  return JUNK_TIERS.find((entry) => entry.tier === tier)?.threshold;
}
