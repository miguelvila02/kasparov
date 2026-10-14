// Runs the engine in a background thread, so the page never freezes.
//
// The page sends: the starting FEN + every move played so far + the time budget.
// The worker rebuilds the game (so repetition detection has the full history),
// optionally reviews the user's last move, thinks, plays, and explains its move.

import init, { WasmGame } from "engine";

export type SearchRequest = {
  id: number; // lets the page ignore answers from a game that was abandoned
  startFen: string;
  moves: string[]; // UCI moves played since startFen
  timeMs: number; // the bot's own thinking time
  review: boolean; // review the last move in `moves` (the user's)?
  reviewMs: number; // time for each of the two review searches
};

export type Verdict = "best" | "good" | "inaccuracy" | "mistake" | "blunder";

export type SearchResponse = {
  id: number;
  move: string | null;
  depth: number;
  nodes: number;
  score: number; // centipawns, from White's point of view
  pv: string; // the line the engine expects, in UCI
  explanation: string;
  arrows: [string, string][]; // bot's move, expected reply, bot's follow-up
  review: { verdict: Verdict; text: string } | null;
};

type CoachJson = Omit<SearchResponse, "id">;

const ready = init();

self.onmessage = async (e: MessageEvent<SearchRequest>) => {
  await ready;
  const { id, startFen, moves, timeMs, review, reviewMs } = e.data;

  const g = new WasmGame(startFen);
  try {
    for (const m of moves) g.play(m);
    const json = g.coach_turn(review, reviewMs, timeMs);
    const res: SearchResponse = json
      ? { id, ...(JSON.parse(json) as CoachJson) }
      : { id, move: null, depth: 0, nodes: 0, score: 0, pv: "", explanation: "", arrows: [], review: null };
    (self as unknown as Worker).postMessage(res);
  } finally {
    g.free();
  }
};