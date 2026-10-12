import { useEffect, useRef, useState } from "react";
import init, { WasmGame } from "engine";
import { Chessboard } from "react-chessboard";

// Load the engine once, when this file is first imported
const engineReady = init();

const STATUS_TEXT: Record<string, string> = {
  checkmate_white: "Checkmate: White wins",
  checkmate_black: "Checkmate: Black wins",
  stalemate: "Draw by stalemate",
  fifty_moves: "Draw by the 50-move rule",
  repetition: "Draw by threefold repetition",
  insufficient: "Draw by insufficient material",
};

function formatScore(cp: number) {
  if (Math.abs(cp) > 29000) {
    const moves = Math.ceil((30000 - Math.abs(cp)) / 2);
    return `${cp > 0 ? "White" : "Black"} mates in ${moves}`;
  }
  return `${cp >= 0 ? "+" : ""}${(cp / 100).toFixed(2)}`;
}

export default function App() {
  // ---------- State (hooks must stay inside App) ----------
  const game = useRef<WasmGame | null>(null);
  const botTimer = useRef<number | null>(null);
  const thinkingRef = useRef(false);

  const [fen, setFen] = useState("");
  const [status, setStatus] = useState("ongoing");
  const [evalCp, setEvalCp] = useState(0);
  const [pendingPromotion, setPendingPromotion] = useState<string | null>(null);
  const [thinking, setThinking] = useState(false);
  const [info, setInfo] = useState("");

  useEffect(() => {
    engineReady.then(() => {
      if (!game.current) game.current = new WasmGame();
      sync();
    });
  }, []);

  // ---------- Helpers ----------
  function sync() {
    const g = game.current;
    if (!g) return;
    setFen(g.fen());
    setStatus(g.status());
    setEvalCp(g.evaluation());
  }

  function finishThinking() {
    thinkingRef.current = false;
    setThinking(false);
  }

  function afterUserMove() {
    sync();
    if (botTimer.current !== null) return;
    thinkingRef.current = true;
    setThinking(true);
    // Short pause so "Bot is thinking…" is drawn before the search freezes the page
    botTimer.current = window.setTimeout(() => {
      botTimer.current = null;
      botMove();
    }, 50);
  }

  function botMove() {
    const g = game.current;
    if (!g || g.status() !== "ongoing" || g.side_to_move() !== "b") {
      finishThinking();
      return;
    }

    const delay = 1000 + Math.random() * 4000; // 1–5 seconds
    const start = performance.now();
    g.bot_move(Math.max(100, delay - 250)); // the actual thinking
    const remaining = Math.max(0, delay - (performance.now() - start));

    botTimer.current = window.setTimeout(() => {
      botTimer.current = null;
      setInfo(
        `Depth ${g.last_depth()} · ${g.last_nodes().toLocaleString()} positions · ${formatScore(g.last_score())}`
      );
      finishThinking();
      sync();
    }, remaining);
  }

  // ---------- User actions ----------
  function onPieceDrop({
    sourceSquare,
    targetSquare,
  }: {
    sourceSquare: string;
    targetSquare: string | null;
  }) {
    const g = game.current;
    if (!g || !targetSquare || pendingPromotion || thinkingRef.current) return false;
    if (g.status() !== "ongoing" || g.side_to_move() !== "w") return false;

    const uci = sourceSquare + targetSquare;
    const legal = g.legal_moves();

    // A promotion: the move only exists with a piece letter at the end
    if (legal.includes(uci + "q")) {
      setPendingPromotion(uci);
      return false; // the pawn waits on its square until a piece is chosen
    }

    if (!legal.includes(uci)) return false; // illegal: snap back
    g.play(uci);
    afterUserMove();
    return true;
  }

  function choosePromotion(piece: "q" | "r" | "b" | "n") {
    const g = game.current;
    if (!g || !pendingPromotion) return;
    g.play(pendingPromotion + piece);
    setPendingPromotion(null);
    afterUserMove();
  }

  function newGame() {
    if (botTimer.current !== null) {
      clearTimeout(botTimer.current);
      botTimer.current = null;
    }
    finishThinking();
    setInfo("");
    setPendingPromotion(null);
    try {
      game.current?.free();
    } catch {
      // the old engine object was already broken; ignore it
    }
    game.current = new WasmGame();
    sync();
  }

  // ---------- Page ----------
  if (!fen) return <p>Loading engine…</p>;

  return (
    <div style={{ maxWidth: 480, margin: "2rem auto", fontFamily: "sans-serif" }}>
      <div style={{ position: "relative" }}>
        <Chessboard options={{ position: fen, onPieceDrop }} />

        {pendingPromotion && (
          <div
            style={{
              position: "absolute",
              inset: 0,
              background: "rgba(0, 0, 0, 0.55)",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 12,
              zIndex: 10,
            }}
          >
            <p style={{ color: "white", margin: 0, fontSize: 18 }}>Promote to:</p>
            <div style={{ display: "flex", gap: 8 }}>
              {(
                [
                  ["q", "♕", "Queen"],
                  ["r", "♖", "Rook"],
                  ["b", "♗", "Bishop"],
                  ["n", "♘", "Knight"],
                ] as const
              ).map(([letter, symbol, name]) => (
                <button
                  key={letter}
                  title={name}
                  onClick={() => choosePromotion(letter)}
                  style={{
                    fontSize: 44,
                    width: 72,
                    height: 72,
                    cursor: "pointer",
                    borderRadius: 8,
                    border: "none",
                    background: "#f0d9b5",
                  }}
                >
                  {symbol}
                </button>
              ))}
            </div>
            <button onClick={() => setPendingPromotion(null)}>Cancel</button>
          </div>
        )}
      </div>

      <p>{STATUS_TEXT[status] ?? (thinking ? "Bot is thinking…" : "Your move")}</p>
      <p>Evaluation: {formatScore(evalCp)}</p>
      {info && <p style={{ fontSize: 14, opacity: 0.7 }}>Bot: {info}</p>}
      <button onClick={newGame}>New game</button>
    </div>
  );
}