// Opening book logic. Pure functions, no React, so it is easy to test.
//
// A "book" is a list of lines (move sequences from the start position).
// While the moves played so far are the beginning of at least one line,
// we are "in book" and the next moves of those lines are the book moves.

import { DEFENSES, WHITE_OPENINGS } from "./openings";
import type { BookLine, BookMove, Opening } from "./openings";
import NAMES from "./opening-names.json";

export type BookChoice = BookMove & { lineName: string };

/** All lines of one opening, or of every curated opening (for "no preference"). */
export function bookLines(opening: Opening | null): BookLine[] {
  if (opening) return opening.lines;
  return [...WHITE_OPENINGS, ...DEFENSES].flatMap((o) => o.lines);
}

function startsWith(line: BookLine, played: string[]): boolean {
  if (line.moves.length < played.length) return false;
  return played.every((uci, i) => line.moves[i].uci === uci);
}

/** Book moves available after `played`, one entry per distinct move. Empty = out of book. */
export function nextBookMoves(lines: BookLine[], played: string[]): BookChoice[] {
  const seen = new Map<string, BookChoice>();
  for (const line of lines) {
    if (line.moves.length > played.length && startsWith(line, played)) {
      const m = line.moves[played.length];
      if (!seen.has(m.uci)) seen.set(m.uci, { ...m, lineName: line.name });
    }
  }
  return [...seen.values()];
}

/** Is `played` still inside at least one line (including a line that just ended)? */
export function isInBook(lines: BookLine[], played: string[]): boolean {
  return lines.some((line) => startsWith(line, played));
}

/** Random book move, or null if out of book. */
export function pickBookMove(lines: BookLine[], played: string[], rand = Math.random): BookChoice | null {
  const options = nextBookMoves(lines, played);
  if (options.length === 0) return null;
  return options[Math.floor(rand() * options.length)];
}

/** Name of a position from the Lichess opening database, keyed by the first 4 FEN fields. */
export function openingName(fen: string): string | null {
  const key = fen.split(" ").slice(0, 4).join(" ");
  return (NAMES as Record<string, string>)[key] ?? null;
}

/** If the last move of `played` was a book move, returns it (with its idea). */
export function lastBookMove(lines: BookLine[], played: string[]): BookChoice | null {
  if (played.length === 0) return null;
  const last = played[played.length - 1];
  return nextBookMoves(lines, played.slice(0, -1)).find((m) => m.uci === last) ?? null;
}