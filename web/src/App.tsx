import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import init, { WasmGame } from "engine";
import { Chessboard } from "react-chessboard";
import type { SearchRequest, SearchResponse } from "./engine.worker";

// ---------- Outside the component: constants and plain helpers (no hooks here) ----------

// The engine on the page is only used for the rules (legal moves, status, notation).
// The thinking happens in the worker.
const engineReady = init();

const START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

type Side = "w" | "b";
type Screen = "menu" | "game";

function createWorker() {
  return new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" });
}

function formatScore(cp: number) {
  if (Math.abs(cp) > 29000) {
    const moves = Math.ceil((30000 - Math.abs(cp)) / 2);
    return `${cp > 0 ? "White" : "Black"} mates in ${moves}`;
  }
  return `${cp >= 0 ? "+" : ""}${(cp / 100).toFixed(2)}`;
}

function resultText(status: string, user: Side): string | null {
  switch (status) {
    case "checkmate_white":
      return user === "w" ? "Checkmate: you win!" : "Checkmate: the bot wins.";
    case "checkmate_black":
      return user === "b" ? "Checkmate: you win!" : "Checkmate: the bot wins.";
    case "stalemate":
      return "Draw by stalemate";
    case "fifty_moves":
      return "Draw by the 50-move rule";
    case "repetition":
      return "Draw by threefold repetition";
    case "insufficient":
      return "Draw by insufficient material";
    default:
      return null;
  }
}

const LAST_MOVE: CSSProperties = { background: "rgba(255, 213, 79, 0.45)" };

// ---------- The component ----------

export default function App() {
  // Refs: values the callbacks need right now, without waiting for a re-render
  const game = useRef<WasmGame | null>(null);
  const worker = useRef<Worker | null>(null);
  const userSideRef = useRef<Side>("w");
  const movesRef = useRef<string[]>([]);
  const requestId = useRef(0);
  const pending = useRef<{ id: number; delay: number; sentAt: number } | null>(null);
  const botTimer = useRef<number | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // State: what's drawn on the screen
  const [engineLoaded, setEngineLoaded] = useState(false);
  const [screen, setScreen] = useState<Screen>("menu");
  const [userSide, setUserSide] = useState<Side>("w");
  const [fen, setFen] = useState(START_FEN);
  const [status, setStatus] = useState("ongoing");
  const [evalCp, setEvalCp] = useState(0);
  const [sanList, setSanList] = useState<string[]>([]);
  const [lastMove, setLastMove] = useState<{ from: string; to: string } | null>(null);
  const [pendingPromotion, setPendingPromotion] = useState<string | null>(null);
  const [thinking, setThinking] = useState(false);
  const [info, setInfo] = useState("");

  useEffect(() => {
    engineReady.then(() => setEngineLoaded(true));
    return () => {
      worker.current?.terminate();
      worker.current = null;
    };
  }, []);

  // Keep the move list scrolled to the latest move
  useEffect(() => {
    listRef.current?.scrollTo({ top: listRef.current.scrollHeight });
  }, [sanList]);

  // ---------- Engine worker ----------

  function getWorker() {
    if (!worker.current) {
      const w = createWorker();
      w.onmessage = (e: MessageEvent<SearchResponse>) => onBotResult(e.data);
      w.onerror = (e) => {
        console.error("Engine worker error:", e.message);
        pending.current = null;
        setThinking(false);
      };
      worker.current = w;
    }
    return worker.current;
  }

  /** Stops any bot move in progress (search or waiting period). */
  function cancelBot() {
    if (botTimer.current !== null) {
      clearTimeout(botTimer.current);
      botTimer.current = null;
    }
    if (pending.current) {
      // A search is still running: kill the worker so it doesn't keep thinking
      worker.current?.terminate();
      worker.current = null;
      pending.current = null;
    }
    requestId.current++;
    setThinking(false);
  }

  function requestBotMove() {
    const g = game.current;
    if (!g || g.status() !== "ongoing" || g.side_to_move() === userSideRef.current) return;

    const delay = 1000 + Math.random() * 4000; // 1–5 seconds of "thinking"
    const id = ++requestId.current;
    pending.current = { id, delay, sentAt: performance.now() };
    setThinking(true);

    const req: SearchRequest = {
      id,
      startFen: START_FEN,
      moves: [...movesRef.current],
      timeMs: Math.max(100, delay - 250),
    };
    getWorker().postMessage(req);
  }

  function onBotResult(res: SearchResponse) {
    const p = pending.current;
    if (!p || res.id !== p.id) return; // answer from an abandoned game

    // If the search finished early, wait out the rest of the delay
    const remaining = Math.max(0, p.delay - (performance.now() - p.sentAt));
    botTimer.current = window.setTimeout(() => {
      botTimer.current = null;
      pending.current = null;
      if (res.move) applyMove(res.move);
      setInfo(`Depth ${res.depth} · ${res.nodes.toLocaleString()} positions · ${formatScore(res.score)}`);
      setThinking(false);
    }, remaining);
  }

  // ---------- Game ----------

  function sync() {
    const g = game.current;
    if (!g) return;
    setFen(g.fen());
    setStatus(g.status());
    setEvalCp(g.evaluation());
  }

  /** Plays a UCI move on the page's game and records it. */
  function applyMove(uci: string): boolean {
    const g = game.current;
    if (!g) return false;
    const san = g.san(uci); // must be computed BEFORE the move is played
    if (!san || !g.play(uci)) return false;

    movesRef.current.push(uci);
    setSanList((prev) => [...prev, san]);
    setLastMove({ from: uci.slice(0, 2), to: uci.slice(2, 4) });
    sync();
    return true;
  }

  function startGame(side: Side) {
    cancelBot();
    try {
      game.current?.free();
    } catch {
      // the old engine object was already broken; ignore it
    }
    game.current = new WasmGame();
    movesRef.current = [];
    userSideRef.current = side;

    setUserSide(side);
    setSanList([]);
    setLastMove(null);
    setPendingPromotion(null);
    setInfo("");
    setScreen("game");
    sync();

    if (side === "b") requestBotMove(); // the bot plays White, so it moves first
  }

  function backToMenu() {
    cancelBot();
    setPendingPromotion(null);
    setScreen("menu");
  }

  // ---------- User input ----------

  function onPieceDrop({
    sourceSquare,
    targetSquare,
  }: {
    sourceSquare: string;
    targetSquare: string | null;
  }) {
    const g = game.current;
    if (!g || !targetSquare || pendingPromotion || pending.current) return false;
    if (g.status() !== "ongoing" || g.side_to_move() !== userSideRef.current) return false;

    const uci = sourceSquare + targetSquare;
    const legal = g.legal_moves();

    // A promotion only exists with a piece letter at the end
    if (legal.includes(uci + "q")) {
      setPendingPromotion(uci);
      return false; // the pawn waits on its square until a piece is chosen
    }

    if (!legal.includes(uci) || !applyMove(uci)) return false; // illegal: snap back
    requestBotMove();
    return true;
  }

  function choosePromotion(piece: "q" | "r" | "b" | "n") {
    if (!pendingPromotion) return;
    const ok = applyMove(pendingPromotion + piece);
    setPendingPromotion(null);
    if (ok) requestBotMove();
  }

  // ---------- Screens ----------

  if (!engineLoaded) {
    return <p style={{ textAlign: "center", marginTop: "4rem", fontFamily: "sans-serif" }}>Loading engine…</p>;
  }

  if (screen === "menu") {
    const big: CSSProperties = {
      display: "flex",
      flexDirection: "column",
      alignItems: "center",
      gap: 4,
      width: 170,
      padding: "20px 0",
      fontSize: 18,
      cursor: "pointer",
      borderRadius: 12,
      border: "2px solid #b58863",
      background: "#f0d9b5",
      color: "#222",
    };
    return (
      <div style={{ maxWidth: 520, margin: "4rem auto", padding: "0 16px", textAlign: "center", fontFamily: "sans-serif" }}>
        <h1 style={{ marginBottom: 8 }}>Kasparov</h1>
        <p style={{ opacity: 0.7, marginTop: 0 }}>Choose your side</p>
        <div style={{ display: "flex", gap: 16, justifyContent: "center", flexWrap: "wrap", marginTop: 24 }}>
          <button style={big} onClick={() => startGame("w")}>
            <span style={{ fontSize: 56 }}>♔</span>
            Play as White
          </button>
          <button style={big} onClick={() => startGame("b")}>
            <span style={{ fontSize: 56 }}>♚</span>
            Play as Black
          </button>
        </div>
      </div>
    );
  }

  const result = resultText(status, userSide);
  const turnText = result ?? (thinking ? "Bot is thinking…" : "Your move");
  const squareStyles = lastMove ? { [lastMove.from]: LAST_MOVE, [lastMove.to]: LAST_MOVE } : {};
  const promoPieces =
    userSide === "w"
      ? ([["q", "♕", "Queen"], ["r", "♖", "Rook"], ["b", "♗", "Bishop"], ["n", "♘", "Knight"]] as const)
      : ([["q", "♛", "Queen"], ["r", "♜", "Rook"], ["b", "♝", "Bishop"], ["n", "♞", "Knight"]] as const);

  const rows: [number, string, string | undefined][] = [];
  for (let i = 0; i < sanList.length; i += 2) rows.push([i / 2 + 1, sanList[i], sanList[i + 1]]);

  return (
    <div
      style={{
        display: "flex",
        flexWrap: "wrap",
        gap: 24,
        justifyContent: "center",
        alignItems: "flex-start",
        margin: "2rem auto",
        padding: "0 16px",
        maxWidth: 820,
        fontFamily: "sans-serif",
      }}
    >
      {/* Board */}
      <div style={{ position: "relative", width: "min(480px, 100%)" }}>
        <Chessboard
          options={{
            position: fen,
            onPieceDrop,
            boardOrientation: userSide === "w" ? "white" : "black",
            squareStyles,
          }}
        />

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
              {promoPieces.map(([letter, symbol, name]) => (
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
                    color: "#222",
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

      {/* Side panel */}
      <div style={{ width: 260, display: "flex", flexDirection: "column", gap: 12 }}>
        <div>
          <div style={{ fontSize: 13, opacity: 0.6 }}>
            You play {userSide === "w" ? "White" : "Black"}
          </div>
          <div style={{ fontSize: 20, fontWeight: 600 }}>{turnText}</div>
        </div>

        <div style={{ fontSize: 14 }}>Evaluation: {formatScore(evalCp)}</div>
        {info && <div style={{ fontSize: 13, opacity: 0.7 }}>Bot: {info}</div>}

        <div
          ref={listRef}
          style={{
            maxHeight: 320,
            overflowY: "auto",
            border: "1px solid rgba(128, 128, 128, 0.35)",
            borderRadius: 8,
            padding: "6px 10px",
            fontFamily: "monospace",
            fontSize: 15,
          }}
        >
          {rows.length === 0 && <div style={{ opacity: 0.5 }}>No moves yet</div>}
          {rows.map(([n, white, black]) => (
            <div key={n} style={{ display: "grid", gridTemplateColumns: "36px 1fr 1fr", padding: "2px 0" }}>
              <span style={{ opacity: 0.5 }}>{n}.</span>
              <span>{white}</span>
              <span>{black ?? ""}</span>
            </div>
          ))}
        </div>

        <button onClick={backToMenu}>{result ? "New game" : "Back to menu"}</button>
      </div>
    </div>
  );
}