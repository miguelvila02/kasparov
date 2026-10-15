// "User vs CPU": the full game against the bot (Phases 5–9) plus takebacks.
import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { WasmGame } from "engine";
import { Chessboard } from "react-chessboard";
import type { SearchRequest, SearchResponse, Verdict } from "./engine.worker";
import { WHITE_OPENINGS } from "./openings";
import type { BookLine, Opening } from "./openings";
import { bookLines, isInBook, lastBookMove, openingName, pickBookMove } from "./book";
import {
  ARROW_COLORS,
  CoachPanel,
  LAST_MOVE,
  MoveList,
  PromotionDialog,
  START_FEN,
  card,
  createWorker,
  formatScore,
  gameLayout,
  moveLabel,
  page,
  resultText,
  sidePanel,
} from "./shared";
import type { Arrow, Note, NoteKind, Side } from "./shared";

type Screen = "side" | "openings" | "game";

/** Symbol shown next to a move in the move list. */
const MARKS: Partial<Record<Verdict, string>> = {
  best: " ✓",
  inaccuracy: "?!",
  mistake: "?",
  blunder: "??",
};

/** How many of your moves you can take back in a row. */
const MAX_TAKEBACKS = 4;

export default function CpuGame({ onExit }: { onExit: () => void }) {
  // Refs: values the callbacks need right now, without waiting for a re-render
  const game = useRef<WasmGame | null>(null);
  const worker = useRef<Worker | null>(null);
  const userSideRef = useRef<Side>("w");
  const movesRef = useRef<string[]>([]);
  const namesRef = useRef<(string | null)[]>([]); // opening name after each ply
  const requestId = useRef(0);
  const pending = useRef<{ id: number; delay: number; sentAt: number; kind: "search" | "book" } | null>(null);
  const botTimer = useRef<number | null>(null);
  // Opening book
  const bookRef = useRef<BookLine[]>([]);
  const inBookRef = useRef(false);
  const chosenOpeningRef = useRef<Opening | null>(null);
  const noteId = useRef(0);
  // Was the user's last move a book move? (Book moves are explained, not reviewed.)
  const lastUserMoveWasBook = useRef(false);

  // State: what's drawn on the screen
  const [screen, setScreen] = useState<Screen>("side");
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
  const [takebacksLeft, setTakebacksLeft] = useState(MAX_TAKEBACKS);

  useEffect(() => {
    return () => {
      if (botTimer.current !== null) clearTimeout(botTimer.current);
      worker.current?.terminate();
      worker.current = null;
      try {
        game.current?.free();
      } catch {
        // already freed
      }
    };
  }, []);

  function addNote(kind: NoteKind, text: string, ply?: number) {
    const id = ++noteId.current;
    setNotes((prev) => [...prev, { id, kind, text, ply }]);
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
            addNote("book", `${moveLabel(ply, choice.san)} (book) ${choice.idea}`, ply);
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
        addNote("info", "End of the prepared line. From here on, the bot thinks for itself.", movesRef.current.length);
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
    if (!p || res.id !== p.id) return; // answer from an abandoned game or a takeback

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
          addNote(res.review.verdict, res.review.text, userPly);
        }
      }

      // 2. The bot's move and its explanation
      const botPly = movesRef.current.length;
      if (res.move && applyMove(res.move)) {
        if (res.explanation) addNote("engine", res.explanation, botPly);
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
    setFen(g.fen());
    setStatus(g.status());
    setEvalCp(g.evaluation());
    // The opening name: the most recent position that has a name
    const named = [...namesRef.current].reverse().find((n) => n !== null);
    setPositionName(named ?? null);
  }

  function highlightLast() {
    const last = movesRef.current[movesRef.current.length - 1];
    setLastMove(last ? { from: last.slice(0, 2), to: last.slice(2, 4) } : null);
  }

  /** Plays a UCI move on the page's game and records it. */
  function applyMove(uci: string): boolean {
    const g = game.current;
    if (!g) return false;
    const san = g.san(uci); // must be computed BEFORE the move is played
    if (!san || !g.play(uci)) return false;

    movesRef.current.push(uci);
    namesRef.current.push(openingName(g.fen()));
    setSanList((prev) => [...prev, san]);
    highlightLast();
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
        addNote("book", `${moveLabel(ply, m.san)} (book) ${m.idea}`, ply);
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
          " From here on, the bot thinks for itself.",
        ply
      );
    }
  }

  /** How many moves the user has played so far. */
  function userMoveCount() {
    const n = movesRef.current.length;
    // White plays plies 0, 2, 4…; Black plays plies 1, 3, 5…
    return userSideRef.current === "w" ? Math.ceil(n / 2) : Math.floor(n / 2);
  }

  /** Takes back the user's last move (and the bot's reply, if it already came). */
  function takeBack() {
    const g = game.current;
    if (!g || takebacksLeft <= 0 || userMoveCount() === 0) return;
    cancelBot();
    setPendingPromotion(null);

    // Remove plies until we've removed one user move and it's the user's turn again
    let removedUserMove = false;
    while (movesRef.current.length > 0) {
      const lastPly = movesRef.current.length - 1;
      const lastWasUser = (lastPly % 2 === 0) === (userSideRef.current === "w");
      g.undo();
      movesRef.current.pop();
      namesRef.current.pop();
      if (lastWasUser) {
        removedUserMove = true;
        break;
      }
    }
    if (!removedUserMove) return;

    const len = movesRef.current.length;
    setSanList((prev) => prev.slice(0, len));
    setMarks((prev) => Object.fromEntries(Object.entries(prev).filter(([k]) => Number(k) < len)));
    setNotes((prev) => prev.filter((n) => n.ply === undefined || n.ply < len));
    setArrows([]);
    setInfo("");
    highlightLast();

    // Back in the opening book? (e.g. the takeback removed the move that left it)
    inBookRef.current = isInBook(bookRef.current, movesRef.current);
    lastUserMoveWasBook.current = false;

    setTakebacksLeft((t) => t - 1);
    sync();
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
    namesRef.current = [];
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
    setMarks({});
    setArrows([]);
    setTakebacksLeft(MAX_TAKEBACKS);
    setNotes(
      opening
        ? [{ id: 0, kind: "info", text: `The bot will play the ${opening.name} (${opening.firstMoves}). ${opening.description}` }]
        : []
    );
    setScreen("game");
    sync();

    if (side === "b") requestBotMove(); // the bot plays White, so it moves first
  }

  function leaveGame() {
    cancelBot();
    setPendingPromotion(null);
    setScreen("side");
  }

  // ---------- User input ----------

  /** Called after any successful user move. */
  function afterUserMove(san: string) {
    setArrows([]);
    setTakebacksLeft(MAX_TAKEBACKS); // the limit is per streak of takebacks
    checkUserMoveAgainstBook(san);
    requestBotMove();
  }

  function onPieceDrop({ sourceSquare, targetSquare }: { sourceSquare: string; targetSquare: string | null }) {
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
    afterUserMove(san);
    return true;
  }

  function choosePromotion(piece: "q" | "r" | "b" | "n") {
    const g = game.current;
    if (!g || !pendingPromotion) return;
    const uci = pendingPromotion + piece;
    const san = g.san(uci) ?? uci;
    const ok = applyMove(uci);
    setPendingPromotion(null);
    if (ok) afterUserMove(san);
  }

  // ---------- Screens ----------

  if (screen === "side") {
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
      <div style={{ ...page, maxWidth: 520, margin: "3rem auto", textAlign: "center" }}>
        <div style={{ textAlign: "left" }}>
          <button onClick={onExit}>← Menu</button>
        </div>
        <h2 style={{ marginBottom: 8 }}>User vs CPU</h2>
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
    return (
      <div style={{ ...page, maxWidth: 820, margin: "2.5rem auto" }}>
        <button onClick={() => setScreen("side")}>← Back</button>
        <h2 style={{ marginBottom: 4 }}>Which opening should the bot play?</h2>
        <p style={{ opacity: 0.7, marginTop: 0 }}>
          The bot plays White and follows the opening you pick, explaining each move. If you leave its prepared lines, it
          tells you and starts thinking for itself.
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
  const canTakeBack = takebacksLeft > 0 && sanList.length > 0 && userMoveCount() > 0;

  return (
    <div style={gameLayout}>
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
          <PromotionDialog side={userSide} onChoose={choosePromotion} onCancel={() => setPendingPromotion(null)} />
        )}
      </div>

      {/* Side panel */}
      <div style={sidePanel}>
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

        <MoveList sans={sanList} marks={marks} />

        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <button onClick={takeBack} disabled={!canTakeBack} title="Undo your last move and the bot's reply">
            ↶ Take back
          </button>
          <span style={{ fontSize: 13, opacity: 0.7 }}>
            {takebacksLeft} of {MAX_TAKEBACKS} left in a row
          </span>
        </div>

        <CoachPanel notes={notes} />

        <div style={{ display: "flex", gap: 8 }}>
          <button onClick={leaveGame} style={{ flex: 1 }}>
            {result ? "New game" : "Change side / opening"}
          </button>
          <button onClick={onExit} style={{ flex: 1 }}>
            Main menu
          </button>
        </div>
      </div>
    </div>
  );
}