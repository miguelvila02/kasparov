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

  function onPieceDrop(args: { sourceSquare: string; targetSquare: string | null }) {
    console.log("drop", args, game.current?.side_to_move(), game.current?.legal_moves());

    const { sourceSquare, targetSquare } = args;
    const g = game.current;
    if (!g || !targetSquare || g.status() !== "ongoing" || g.side_to_move() !== "w") return false;

    const uci = sourceSquare + targetSquare;
    const ok = g.play(uci) || g.play(uci + "q"); // auto-queen on promotion
    if (!ok) return false; // illegal move: the piece snaps back

    sync();
    setTimeout(botMove, 500);
    return true;
  }

  function newGame() {
    game.current?.free();
    game.current = new WasmGame();
    sync();
  }

  if (!fen) return <p>Loading engine…</p>;

  return (
    <div style={{ maxWidth: 480, margin: "2rem auto", fontFamily: "sans-serif" }}>
      {/* react-chessboard v5 API; on v4 use <Chessboard position={fen} onPieceDrop={(s, t) => ...} /> */}
      <Chessboard options={{ position: fen, onPieceDrop }} />
      <p>{STATUS_TEXT[status] ?? (game.current?.side_to_move() === "w" ? "Your move" : "Bot is moving…")}</p>
      <button onClick={newGame}>New game</button>
    </div>
  );
}