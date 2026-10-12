// Runs the engine search in a background thread, so the page never freezes.
//
// The main page sends: the starting FEN + every move played so far + how long to think.
// The worker rebuilds the game (so repetition detection has the full history),
// searches, and sends back the move it chose plus what it found.

import init, { WasmGame } from "engine";

export type SearchRequest = {
  id: number; // lets the page ignore answers from a game that was abandoned
  startFen: string;
  moves: string[]; // UCI moves played since startFen
  timeMs: number;
};

export type SearchResponse = {
  id: number;
  move: string | null;
  depth: number;
  nodes: number;
  score: number; // centipawns, from White's point of view
  pv: string; // the line the engine expects, in UCI
};

const ready = init();

self.onmessage = async (e: MessageEvent<SearchRequest>) => {
  await ready;
  const { id, startFen, moves, timeMs } = e.data;

  const g = new WasmGame(startFen);
  try {
    for (const m of moves) g.play(m);
    const move = g.bot_move(timeMs) ?? null;

    const res: SearchResponse = {
      id,
      move,
      depth: g.last_depth(),
      nodes: g.last_nodes(),
      score: g.last_score(),
      pv: g.last_pv(),
    };
    (self as unknown as Worker).postMessage(res);
  } finally {
    g.free();
  }
};