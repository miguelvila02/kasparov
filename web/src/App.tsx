import { useEffect, useRef, useState } from "react";
import init, { WasmGame } from "engine";
import { Chessboard } from "react-chessboard";

const engineReady = init(); // runs once, when the module loads


const STATUS_TEXT: Record<string, string> = {
  checkmate_white: "Checkmate: White wins",
  checkmate_black: "Checkmate: Black wins",
  stalemate: "Draw by stalemate",
  fifty_moves: "Draw by the 50-move rule",
  repetition: "Draw by threefold repetition",
  insufficient: "Draw by insufficient material",
};

export default function App() {
  const game = useRef<WasmGame | null>(null);
  const [fen, setFen] = useState("");
  const [status, setStatus] = useState("ongoing");
  const [pendingPromotion, setPendingPromotion] = useState<string | null>(null);


  useEffect(() => {
    engineReady.then(() => {
      game.current = new WasmGame();
      setFen(game.current.fen());
    });
  }, []);

  function sync() {
    const g = game.current!;
    setFen(g.fen());
    setStatus(g.status());
  }

  function botMove() {
    const g = game.current;
    console.log("botMove called", g?.status(), g?.side_to_move());
    if (!g || g.status() !== "ongoing") return;

    try {
      const moves = g.legal_moves();
      console.log("bot legal moves:", moves);
      const m = moves[Math.floor(Math.random() * moves.length)];
      const ok = g.play(m);
      console.log("bot played", m, ok);
      sync();
    } catch (e) {
      console.error("botMove failed:", e);
    }
  }

  function afterUserMove() {
    sync();
    setTimeout(botMove, 500);
  }

  function onPieceDrop({
    sourceSquare,
    targetSquare,
  }: {
    sourceSquare: string;
    targetSquare: string | null;
  }) {
    const g = game.current;
    if (!g || !targetSquare || pendingPromotion) return false;
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
    game.current?.free();
    game.current = new WasmGame();
    setPendingPromotion(null);
    sync();
  }

  if (!fen) return <p>Loading engine…</p>;

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

      <p>{STATUS_TEXT[status] ?? (game.current?.side_to_move() === "w" ? "Your move" : "Bot is moving…")}</p>
      <button onClick={newGame}>New game</button>
    </div>
  );
}