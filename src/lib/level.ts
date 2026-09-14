/**
 * EXP needed to reach `level`: `n³ + 137n² − 77n − 61` (`codes::level_threshold`).
 *
 * Signed because `threshold(0)` is `-61`; levels start at 1, whose threshold is
 * exactly 0.
 */
export function levelThreshold(level: number): number {
  return level ** 3 + 137 * level ** 2 - 77 * level - 61;
}

/** The largest level whose threshold is `<= exp`, minimum 1. */
export function levelFromExp(exp: number): number {
  let level = 1;
  while (levelThreshold(level + 1) <= exp) {
    level += 1;
  }
  return level;
}
