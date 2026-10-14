import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import init, { WasmGame } from "engine";
import { Chessboard } from "react-chessboard";
import type { SearchRequest, SearchResponse, Verdict } from "./engine.worker";
import { WHITE_OPENINGS } from "./openings";
import type { BookLine, Opening } from "./openings";
import { bookLines, isInBook, lastBookMove, openingName, pickBookMove } from "./book";

// ---------- Outside the component: constants and plain helpers (no hooks here) ----------

// The engine on the page is only used for the rules (legal moves, status, notation).
// The thinking happens in the worker.
const engineReady = init();

const START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

type Side = "w" | "b";
type Screen = "menu" | "openings" | "game";
type NoteKind = "book" | "info" | "warn" | "engine" | "inaccuracy" | "mistake" | "blunder";
type Note = { id: number; kind: NoteKind; text: string };
type Arrow = { startSquare: string; endSquare: string; color: string };

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

/** "3.Bc4" or "3...Nf6" for the move at index `ply` (0 = White's first move). */
function moveLabel(ply: number, san: string) {
  const n = Math.floor(ply / 2) + 1;
  return ply % 2 === 0 ? `${n}.${san}` : `${n}...${san}`;
}

const LAST_MOVE: CSSProperties = { background: "rgba(255, 213, 79, 0.45)" };

const NOTE_COLORS: Record<NoteKind, string> = {
  book: "#3b82f6",
  info: "#9ca3af",
  warn: "#f59e0b",
  engine: "#10b981",
  inaccuracy: "#eab308",
  mistake: "#f97316",
  blunder: "#ef4444",
};

/** Symbol shown next to a move in the move list. */
const MARKS: Partial<Record<Verdict, string>> = {
  best: " ✓",
  inaccuracy: "?!",
  mistake: "?",
  blunder: "??",
};

// Plan arrows: the bot's move, the reply it expects, and its follow-up
const ARROW_COLORS = ["rgba(37, 99, 235, 0.85)", "rgba(234, 88, 12, 0.75)", "rgba(37, 99, 235, 0.4)"];

// ---------- The component ----------

export default function App() {
  // Refs: values the callbacks need right now, without waiting for a re-render
  const game = useRef<WasmGame | null>(null);
  const worker = useRef<Worker | null>(null);
  const userSideRef = useRef<Side>("w");
  const movesRef = useRef<string[]>([]);
  const requestId = useRef(0);
  const pending = useRef<{ id: number; delay: number; sentAt: number; kind: "search" | "book" } | null>(null);
  const botTimer = useRef<number | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const notesRef = useRef<HTMLDivElement>(null);
  // Opening book
  const bookRef = useRef<BookLine[]>([]);
  const inBookRef = useRef(false);
  const chosenOpeningRef = useRef<Opening | null>(null);
  const noteId = useRef(0);
  // Was the user's last move a book move? (Book moves are explained, not reviewed.)
  const lastUserMoveWasBook = useRef(false);

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
  const [chosenOpening, setChosenOpening] = useState<Opening | null>(null);
  const [positionName, setPositionName] = useState<string | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [marks, setMarks] = useState<Record<number, string>>({});
  const [arrows, setArrows] = useState<Arrow[]>([]);
  const [showPlan, setShowPlan] = useState(true);

  useEffect(() => {
    engineReady.then(() => setEngineLoaded(true));
    return () => {
      worker.current?.terminate();
      worker.current = null;
    };
  }, []);

  // Keep the move list and the coach notes scrolled to the latest entry
  useEffect(() => {
    listRef.current?.scrollTo({ top: listRef.current.scrollHeight });
  }, [sanList]);
  useEffect(() => {
    notesRef.current?.scrollTo({ top: notesRef.current.scrollHeight });
  }, [notes]);

  function addNote(kind: NoteKind, text: string) {
    const id = ++noteId.current;
    setNotes((prev) => [...prev, { id, kind, text }]);
  }

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
    if (pending.current?.kind === "search") {
      // A search is still running: kill the worker so it doesn't keep thinking
      worker.current?.terminate();
      worker.current = null;
    }
    pending.current = null;
    requestId.current++;
    setThinking(false);
  }

  function requestBotMove() {
    const g = game.current;
    if (!g || g.status() !== "ongoing" || g.side_to_move() === userSideRef.current) return;
    const id = ++requestId.current;
    setThinking(true);

    // 1. Opening book: play the prepared move (a shorter "thinking" pause, 1–2.5 s)
    if (inBookRef.current) {
      const choice = pickBookMove(bookRef.current, movesRef.current);
      if (choice) {
        const delay = 1000 + Math.random() * 1500;
        pending.current = { id, delay, sentAt: performance.now(), kind: "book" };
        botTimer.current = window.setTimeout(() => {
          botTimer.current = null;
          if (pending.current?.id !== id) return;
          pending.current = null;
          const ply = movesRef.current.length;
          if (applyMove(choice.uci)) {
            addNote("book", `${moveLabel(ply, choice.san)} (book) ${choice.idea}`);
            setInfo(`Book move · ${choice.lineName}`);
            setArrows([{ startSquare: choice.uci.slice(0, 2), endSquare: choice.uci.slice(2, 4), color: ARROW_COLORS[0] }]);
          }
          setThinking(false);
        }, delay);
        return;
      }
      // The prepared line has run out
      inBookRef.current = false;
      if (chosenOpeningRef.current) {
        addNote("info", "End of the prepared line. From here on, the bot thinks for itself.");
      }
    }

    // 2. Engine search in the worker: 1–5 s in total, including the review of the user's move
    const delay = 1000 + Math.random() * 4000;
    const review = movesRef.current.length > 0 && !lastUserMoveWasBook.current;
    const reviewMs = review ? Math.min(500, Math.max(150, delay * 0.15)) : 0;
    pending.current = { id, delay, sentAt: performance.now(), kind: "search" };
    const req: SearchRequest = {
      id,
      startFen: START_FEN,
      moves: [...movesRef.current],
      timeMs: Math.max(150, delay - 250 - 3 * reviewMs),
      review,
      reviewMs,
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

      // 1. Review of the user's move (only errors get a note; the best move gets a ✓)
      const userPly = movesRef.current.length - 1;
      if (res.review && userPly >= 0) {
        const mark = MARKS[res.review.verdict];
        if (mark) setMarks((prev) => ({ ...prev, [userPly]: mark }));
        if (res.review.verdict === "inaccuracy" || res.review.verdict === "mistake" || res.review.verdict === "blunder") {
          addNote(res.review.verdict, res.review.text);
        }
      }

      // 2. The bot's move and its explanation
      if (res.move && applyMove(res.move)) {
        if (res.explanation) addNote("engine", res.explanation);
        setArrows(res.arrows.map(([from, to], i) => ({ startSquare: from, endSquare: to, color: ARROW_COLORS[i] ?? ARROW_COLORS[2] })));
      }
      setInfo(`Depth ${res.depth} · ${res.nodes.toLocaleString()} positions · ${formatScore(res.score)}`);
      setThinking(false);
    }, remaining);
  }

  // ---------- Game ----------

  function sync() {
    const g = game.current;
    if (!g) return;
    const f = g.fen();
    setFen(f);
    setStatus(g.status());
    setEvalCp(g.evaluation());
    const name = openingName(f);
    if (name) setPositionName(name); // keep the last known name once we leave theory
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

  /** After the user's move: explain it if it's theory, or announce leaving the book. */
  function checkUserMoveAgainstBook(san: string) {
    lastUserMoveWasBook.current = false;
    if (!inBookRef.current) return;
    const played = movesRef.current;
    const ply = played.length - 1;
    const opening = chosenOpeningRef.current;

    if (isInBook(bookRef.current, played)) {
      const m = lastBookMove(bookRef.current, played);
      if (m) {
        addNote("book", `${moveLabel(ply, m.san)} (book) ${m.idea}`);
        lastUserMoveWasBook.current = true;
      }
      return;
    }

    inBookRef.current = false;
    if (opening) {
      const name = openingName(game.current!.fen());
      addNote(
        "warn",
        `${moveLabel(ply, san)} leaves the ${opening.name} lines prepared for this game.` +
          (name ? ` This position is known as ${name}.` : "") +
          " From here on, the bot thinks for itself."
      );
    }
  }

  function startGame(side: Side, opening: Opening | null) {
    cancelBot();
    try {
      game.current?.free();
    } catch {
      // the old engine object was already broken; ignore it
    }
    game.current = new WasmGame();
    movesRef.current = [];
    userSideRef.current = side;
    chosenOpeningRef.current = opening;
    bookRef.current = bookLines(opening);
    inBookRef.current = true;
    noteId.current = 0;
    lastUserMoveWasBook.current = false;

    setUserSide(side);
    setChosenOpening(opening);
    setSanList([]);
    setLastMove(null);
    setPendingPromotion(null);
    setInfo("");
    setPositionName(null);
    setMarks({});
    setArrows([]);
    setNotes(
      opening
        ? [{ id: 0, kind: "info", text: `The bot will play the ${opening.name} (${opening.firstMoves}). ${opening.description}` }]
        : []
    );
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

    if (!legal.includes(uci)) return false; // illegal: snap back
    const san = g.san(uci) ?? uci;
    if (!applyMove(uci)) return false;
    setArrows([]);
    checkUserMoveAgainstBook(san);
    requestBotMove();
    return true;
  }

  function choosePromotion(piece: "q" | "r" | "b" | "n") {
    const g = game.current;
    if (!g || !pendingPromotion) return;
    const uci = pendingPromotion + piece;
    const san = g.san(uci) ?? uci;
    const ok = applyMove(uci);
    setPendingPromotion(null);
    if (ok) {
      setArrows([]);
      checkUserMoveAgainstBook(san);
      requestBotMove();
    }
  }

  // ---------- Screens ----------

  const page: CSSProperties = { fontFamily: "sans-serif", padding: "0 16px" };

  if (!engineLoaded) {
    return <p style={{ ...page, textAlign: "center", marginTop: "4rem" }}>Loading engine…</p>;
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
      <div style={{ ...page, maxWidth: 520, margin: "4rem auto", textAlign: "center" }}>
        <h1 style={{ marginBottom: 8 }}>Kasparov</h1>
        <p style={{ opacity: 0.7, marginTop: 0 }}>Choose your side</p>
        <div style={{ display: "flex", gap: 16, justifyContent: "center", flexWrap: "wrap", marginTop: 24 }}>
          <button style={big} onClick={() => startGame("w", null)}>
            <span style={{ fontSize: 56 }}>♔</span>
            Play as White
          </button>
          <button style={big} onClick={() => setScreen("openings")}>
            <span style={{ fontSize: 56 }}>♚</span>
            Play as Black
          </button>
        </div>
      </div>
    );
  }

  if (screen === "openings") {
    const card: CSSProperties = {
      textAlign: "left",
      padding: "12px 14px",
      borderRadius: 10,
      border: "1px solid rgba(128, 128, 128, 0.4)",
      background: "transparent",
      color: "inherit",
      cursor: "pointer",
      display: "flex",
      flexDirection: "column",
      gap: 4,
    };
    return (
      <div style={{ ...page, maxWidth: 820, margin: "2.5rem auto" }}>
        <button onClick={() => setScreen("menu")}>← Back</button>
        <h2 style={{ marginBottom: 4 }}>Which opening should the bot play?</h2>
        <p style={{ opacity: 0.7, marginTop: 0 }}>
          The bot plays White and follows the opening you pick, explaining each move. If you leave its
          prepared lines, it tells you and starts thinking for itself.
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))", gap: 12 }}>
          <button style={{ ...card, borderStyle: "dashed" }} onClick={() => startGame("b", null)}>
            <strong>No preference</strong>
            <span style={{ fontSize: 13, opacity: 0.75 }}>The bot picks a different popular opening each game.</span>
          </button>
          {WHITE_OPENINGS.map((o) => (
            <button key={o.id} style={card} onClick={() => startGame("b", o)}>
              <strong>
                {o.name} <span style={{ fontWeight: 400, opacity: 0.6 }}>{o.eco}</span>
              </strong>
              <span style={{ fontFamily: "monospace", fontSize: 13 }}>{o.firstMoves}</span>
              <span style={{ fontSize: 13, opacity: 0.75 }}>{o.description}</span>
            </button>
          ))}
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

  const withMark = (i: number) => (sanList[i] === undefined ? undefined : sanList[i] + (marks[i] ?? ""));
  const rows: [number, string, string | undefined][] = [];
  for (let i = 0; i < sanList.length; i += 2) rows.push([i / 2 + 1, withMark(i)!, withMark(i + 1)]);

  const box: CSSProperties = {
    border: "1px solid rgba(128, 128, 128, 0.35)",
    borderRadius: 8,
    padding: "6px 10px",
    overflowY: "auto",
  };

  return (
    <div
      style={{
        ...page,
        display: "flex",
        flexWrap: "wrap",
        gap: 24,
        justifyContent: "center",
        alignItems: "flex-start",
        margin: "2rem auto",
        maxWidth: 900,
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
            arrows: showPlan ? arrows : [],
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
      <div style={{ width: 320, maxWidth: "100%", display: "flex", flexDirection: "column", gap: 12, textAlign: "left" }}>
        <div>
          <div style={{ fontSize: 13, opacity: 0.6 }}>
            You play {userSide === "w" ? "White" : "Black"}
            {chosenOpening && ` · Bot's opening: ${chosenOpening.name}`}
          </div>
          <div style={{ fontSize: 20, fontWeight: 600 }}>{turnText}</div>
        </div>

        {positionName && (
          <div style={{ fontSize: 14 }}>
            <span style={{ opacity: 0.6 }}>Opening: </span>
            {positionName}
          </div>
        )}
        <div style={{ fontSize: 14 }}>Evaluation: {formatScore(evalCp)}</div>
        {info && <div style={{ fontSize: 13, opacity: 0.7 }}>Bot: {info}</div>}
        <div style={{ fontSize: 13 }}>
          <label style={{ display: "flex", gap: 6, alignItems: "center", cursor: "pointer" }}>
            <input type="checkbox" checked={showPlan} onChange={(e) => setShowPlan(e.target.checked)} />
            Show the bot's plan on the board
          </label>
          <div style={{ marginLeft: 22, opacity: 0.85 }}>
            <span style={{ color: ARROW_COLORS[0] }}>■</span> its move · <span style={{ color: ARROW_COLORS[1] }}>■</span>{" "}
            the reply it expects
          </div>
        </div>

        <div ref={listRef} style={{ ...box, maxHeight: 200, fontFamily: "monospace", fontSize: 15 }}>
          {rows.length === 0 && <div style={{ opacity: 0.5 }}>No moves yet</div>}
          {rows.map(([n, white, black]) => (
            <div key={n} style={{ display: "grid", gridTemplateColumns: "36px 1fr 1fr", padding: "2px 0" }}>
              <span style={{ opacity: 0.5 }}>{n}.</span>
              <span>{white}</span>
              <span>{black ?? ""}</span>
            </div>
          ))}
        </div>

        <div>
          <div style={{ fontSize: 13, opacity: 0.6, marginBottom: 4 }}>Coach</div>
          <div ref={notesRef} style={{ ...box, maxHeight: 300, display: "flex", flexDirection: "column", gap: 8 }}>
            {notes.length === 0 && <div style={{ opacity: 0.5, fontSize: 14 }}>Notes about the moves appear here.</div>}
            {notes.map((n) => (
              <div
                key={n.id}
                style={{ fontSize: 14, lineHeight: 1.4, borderLeft: `3px solid ${NOTE_COLORS[n.kind]}`, paddingLeft: 8 }}
              >
                {n.text}
              </div>
            ))}
          </div>
        </div>

        <button onClick={backToMenu}>{result ? "New game" : "Back to menu"}</button>
      </div>
    </div>
  );
}