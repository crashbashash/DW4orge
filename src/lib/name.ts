/**
 * The longest player name the game stores.
 *
 * Mirrors Rust `dw4core::name::NAME_CHARS` (8). The save writer `FUN_003ee6d0`
 * fills at most eight UTF-16 slots after the `0xFFFF` marker, so the editor's
 * inputs must not offer more.
 */
export const PLAYER_NAME_MAX = 8;
