// "Endgame Academy": endgames from easy to hard. Each lesson is played twice:
// run 1 with hints (the engine shows the best move), run 2 on your own.
// Every move is explained: yours (reviewed by the engine) and the bot's reply.
import { useEffect, useRef, useState } from "react";
import { WasmGame } from "engine";
import { Chessboard } from "react-chessboard";
import type { SearchRequest, SearchResponse, Verdict } from "./engine.worker";
import { ENDGAMES, GOAL_TEXT } from "./endgames";
import type { EndgameLesson, Level } from "./endgames";
import {
  CoachPanel,
  HINT_COLOR,
  LAST_MOVE,
  MoveList,
  NavBar,
  PromotionDialog,
  box,
  createWorker,
  gameLayout,
  page,
  sidePanel,
} from "./shared";
import type { Arrow, Note, NoteKind, Side } from "./shared";

type Run = 1 | 2;
type Progress = Record<string, { r1?: boolean; r2?: boolean }>;
type Outcome = { ok: boolean; text: string } | null;

const LEVELS: Level[] = ["Easy", "Medium", "Hard"];
const PROGRESS_KEY = "kasparov-endgame-progress-v1";
const HINT_MS = 600;
const REVIEW_MS = 250;
/** Below this evaluation (from your side, in centipawns) the win has slipped away. */
const LOST_WIN_CP = 150;
const MAX_USER_MOVES = 50;

const VERDICT_KIND: Record<Verdict, NoteKind> = {
  best: "good",
  good: "good",
  inaccuracy: "inaccuracy",
  mistake: "mistake",
  blunder: "blunder",
};

function loadProgress(): Progress {
  try {
    return JSON.parse(localStorage.getItem(PROGRESS_KEY) ?? "{}") as Progress;
  } catch {
    return {};
  }
}

function saveProgress(p: Progress) {
  try {
    localStorage.setItem(PROGRESS_KEY, JSON.stringify(p));
  } catch {
    // storage unavailable (private window): progress just isn't remembered
  }
}

/** Piece letters on the board part of a FEN. */
function pieces(fen: string) {
  return fen.split(" ")[0].replace(/[0-9/]/g, "");
}

function count(fen: string, letter: string) {
  return pieces(fen).split("").filter((c) => c === letter).length;
}

// ---------- The list of lessons ----------

export default function EndgameAcademy({ onExit }: { onExit: () => void }) {
  const [progress, setProgress] = useState<Progress>(loadProgress);
  const [lesson, setLesson] = useState<EndgameLesson | null>(null);

  function markDone(id: string, run: Run) {
    setProgress((prev) => {
      const next = { ...prev, [id]: { ...prev[id], [run === 1 ? "r1" : "r2"]: true } };
      saveProgress(next);
      return next;
    });
  }

  if (lesson) {
    const idx = ENDGAMES.findIndex((l) => l.id === lesson.id);
    const next = ENDGAMES[idx + 1];
    return (
      <LessonView
        key={lesson.id}
        lesson={lesson}
        progress={progress[lesson.id] ?? {}}
        onDone={(run) => markDone(lesson.id, run)}
        onBack={() => setLesson(null)}
        onHome={onExit}
        onNext={next ? () => setLesson(next) : undefined}
      />
    );
  }

  // A lesson unlocks once the previous one was completed with hints
  const unlocked = (i: number) => i === 0 || !!progress[ENDGAMES[i - 1].id]?.r1;

  return (
    <>
    <NavBar trail={[{ label: "Kasparov", onClick: onExit }, { label: "Endgame Academy" }]} />
    <div style={{ ...page, maxWidth: 760, margin: "2rem auto" }}>
      <h2 style={{ marginBottom: 4 }}>Endgame Academy</h2>
      <p style={{ opacity: 0.7, marginTop: 0 }}>
        In the classical way of teaching chess, students learn endgames early: with few pieces on the board, you see clearly
        what each piece can do. Each lesson is played twice: first with hints, then on your own. Every move is explained.
      </p>
      {LEVELS.map((level) => (
        <div key={level}>
          <h3 style={{ marginBottom: 8 }}>{level}</h3>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {ENDGAMES.map((l, i) => ({ l, i }))
              .filter(({ l }) => l.level === level)
              .map(({ l, i }) => {
                const p = progress[l.id] ?? {};
                const open = unlocked(i);
                return (
                  <button key={l.id} className="choice-card row" disabled={!open} onClick={() => setLesson(l)}>
                    <span>
                      <strong>
                        {i + 1}. {l.title}
                      </strong>
                      <br />
                      <span style={{ fontSize: 13, opacity: 0.75 }}>{GOAL_TEXT[l.goal]}</span>
                    </span>
                    <span style={{ fontSize: 13, whiteSpace: "nowrap", alignSelf: "center" }}>
                      {!open ? "🔒 Locked" : `${p.r1 ? "✓" : "○"} with hints · ${p.r2 ? "✓" : "○"} on your own`}
                    </span>
                  </button>
                );
              })}
          </div>
        </div>
      ))}
    </div>
    </>
  );
}

// ---------- One lesson ----------

function LessonView(props: {
  lesson: EndgameLesson;
  progress: { r1?: boolean; r2?: boolean };
  onDone: (run: Run) => void;
  onBack: () => void;
  onHome: () => void;
  onNext?: () => void;
}) {
  const { lesson, progress, onDone, onBack, onHome, onNext } = props;
  const userSide = lesson.fen.split(" ")[1] as Side;
  const userQueen = userSide === "w" ? "Q" : "q";
  const userRookOrQueen = userSide === "w" ? /[QR]/ : /[qr]/;
  const oppPieces = userSide === "w" ? /[pnbrq]/ : /[PNBRQ]/;
  const startQueens = count(lesson.fen, userQueen);

  const game = useRef<WasmGame | null>(null);
  const worker = useRef<Worker | null>(null);
  const movesRef = useRef<string[]>([]);
  const requestId = useRef(0);
  const pending = useRef<{ id: number; kind: "hint" | "bot"; delay: number; sentAt: number } | null>(null);
  const timer = useRef<number | null>(null);
  const noteId = useRef(0);
  const runRef = useRef<Run>(progress.r1 ? 2 : 1);
  const finished = useRef(false);

  const [run, setRun] = useState<Run>(runRef.current);
  const [fen, setFen] = useState(lesson.fen);
  const [sans, setSans] = useState<string[]>([]);
  const [lastMove, setLastMove] = useState<{ from: string; to: string } | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [hint, setHint] = useState<Arrow | null>(null);
  const [thinking, setThinking] = useState(false);
  const [outcome, setOutcome] = useState<Outcome>(null);
  const [pendingPromotion, setPendingPromotion] = useState<string | null>(null);

  useEffect(() => {
    start(runRef.current);
    return () => {
      if (timer.current !== null) clearTimeout(timer.current);
      worker.current?.terminate();
      worker.current = null;
      try {
        game.current?.free();
      } catch {
        // already freed
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function addNote(kind: NoteKind, text: string) {
    const id = ++noteId.current;
    setNotes((prev) => [...prev, { id, kind, text }]);
  }

  // ---------- Worker ----------

  function getWorker() {
    if (!worker.current) {
      const w = createWorker();
      w.onmessage = (e: MessageEvent<SearchResponse>) => onResult(e.data);
      w.onerror = (e) => {
        console.error("Engine worker error:", e.message);
        pending.current = null;
        setThinking(false);
      };
      worker.current = w;
    }
    return worker.current;
  }

  function send(kind: "hint" | "bot", timeMs: number, review: boolean, delay: number) {
    const id = ++requestId.current;
    pending.current = { id, kind, delay, sentAt: performance.now() };
    const req: SearchRequest = {
      id,
      startFen: lesson.fen,
      moves: [...movesRef.current],
      timeMs,
      review,
      reviewMs: review ? REVIEW_MS : 0,
    };
    getWorker().postMessage(req);
  }

  function cancelAll() {
    if (timer.current !== null) {
      clearTimeout(timer.current);
      timer.current = null;
    }
    if (pending.current) {
      worker.current?.terminate();
      worker.current = null;
      pending.current = null;
    }
    requestId.current++;
    setThinking(false);
  }

  function requestHint() {
    if (runRef.current !== 1 || finished.current) return;
    send("hint", HINT_MS, false, 0);
  }

  function requestBotMove() {
    setThinking(true);
    const delay = 1000 + Math.random() * 1500; // 1–2.5 s, including the review of your move
    send("bot", Math.max(200, delay * 0.6), true, delay);
  }

  function onResult(res: SearchResponse) {
    const p = pending.current;
    if (!p || res.id !== p.id) return; // an answer nobody is waiting for any more

    if (p.kind === "hint") {
      pending.current = null;
      if (!res.move || finished.current) return;
      setHint({ startSquare: res.move.slice(0, 2), endSquare: res.move.slice(2, 4), color: HINT_COLOR });
      addNote("hint", `Hint: ${res.explanation}`);
      return;
    }

    // The bot's reply: wait out the rest of its "thinking" time
    const remaining = Math.max(0, p.delay - (performance.now() - p.sentAt));
    timer.current = window.setTimeout(() => {
      timer.current = null;
      pending.current = null;
      setThinking(false);
      if (res.review) addNote(VERDICT_KIND[res.review.verdict], res.review.text);
      if (!res.move || !applyMove(res.move)) return;
      if (res.explanation) addNote("engine", res.explanation);

      // Did the win slip away? (Ignore forced replies: the engine doesn't search those.)
      const userScore = userSide === "w" ? res.score : -res.score;
      if (res.depth > 0 && userScore < LOST_WIN_CP && !checkEnd()) {
        fail("That let the win slip away: the engine now thinks the position is no longer winning for you.");
        return;
      }
      if (!checkEnd()) requestHint();
    }, remaining);
  }

  // ---------- Game ----------

  function applyMove(uci: string): boolean {
    const g = game.current;
    if (!g) return false;
    const san = g.san(uci);
    if (!san || !g.play(uci)) return false;
    movesRef.current.push(uci);
    setSans((prev) => [...prev, san]);
    setLastMove({ from: uci.slice(0, 2), to: uci.slice(2, 4) });
    setFen(g.fen());
    return true;
  }

  function start(r: Run) {
    cancelAll();
    try {
      game.current?.free();
    } catch {
      // already freed
    }
    game.current = new WasmGame(lesson.fen);
    movesRef.current = [];
    runRef.current = r;
    finished.current = false;
    noteId.current = 0;
    setRun(r);
    setFen(lesson.fen);
    setSans([]);
    setLastMove(null);
    setHint(null);
    setOutcome(null);
    setPendingPromotion(null);
    setNotes([
      {
        id: 0,
        kind: "info",
        text:
          (r === 1
            ? "Run 1 of 2: the purple arrow shows the best move, with an explanation. "
            : "Run 2 of 2: no hints this time. Use what you learned! ") + GOAL_TEXT[lesson.goal],
      },
    ]);
    requestHint();
  }

  function succeed(text: string) {
    finished.current = true;
    cancelAll();
    setHint(null);
    onDone(runRef.current);
    setOutcome({ ok: true, text });
  }

  function fail(text: string) {
    finished.current = true;
    cancelAll();
    setHint(null);
    setOutcome({ ok: false, text });
  }

  /** Checks whether the lesson is over (success or failure). Returns true if it is. */
  function checkEnd(): boolean {
    const g = game.current;
    if (!g || finished.current) return finished.current;
    const status = g.status();
    const f = g.fen();
    const userWon = status === (userSide === "w" ? "checkmate_white" : "checkmate_black");

    if (userWon) {
      succeed(lesson.goal === "mate" ? "Checkmate! Lesson goal reached." : "Checkmate! Even better than the goal.");
      return true;
    }
    if (status.startsWith("checkmate")) {
      fail("You were checkmated.");
      return true;
    }
    if (status !== "ongoing") {
      const why: Record<string, string> = {
        stalemate: "Stalemate! The enemy king had no legal move but wasn't in check, so it's a draw.",
        insufficient: "Draw: there isn't enough material left to checkmate.",
        repetition: "Draw by repetition: the same position appeared three times.",
        fifty_moves: "Draw by the 50-move rule.",
      };
      fail(why[status] ?? "The game ended in a draw.");
      return true;
    }
    // The goal checks run when it's your turn again (after the bot's reply)
    if (g.side_to_move() === userSide) {
      if (lesson.goal === "promote" && count(f, userQueen) > startQueens) {
        succeed("Your new queen is safe. Lesson goal reached!");
        return true;
      }
      if (lesson.goal === "winMaterial" && !oppPieces.test(pieces(f)) && userRookOrQueen.test(pieces(f))) {
        succeed("Only the enemy king is left. You already know how to mate from here. Lesson goal reached!");
        return true;
      }
    }
    const userMoves = Math.ceil(movesRef.current.length / 2);
    if (userMoves >= MAX_USER_MOVES) {
      fail(`That took more than ${MAX_USER_MOVES} moves. In a real game the 50-move rule would make it a draw.`);
      return true;
    }
    return false;
  }

  // ---------- Your moves ----------

  function afterUserMove() {
    setHint(null);
    requestId.current++; // an unfinished hint is no longer wanted
    if (pending.current?.kind === "hint") pending.current = null;
    if (!checkEnd()) requestBotMove();
  }

  function onPieceDrop({ sourceSquare, targetSquare }: { sourceSquare: string; targetSquare: string | null }) {
    const g = game.current;
    if (!g || !targetSquare || outcome || pendingPromotion || thinking) return false;
    if (g.side_to_move() !== userSide) return false;
    const uci = sourceSquare + targetSquare;
    const legal = g.legal_moves();
    if (legal.includes(uci + "q")) {
      setPendingPromotion(uci);
      return false;
    }
    if (!legal.includes(uci) || !applyMove(uci)) return false;
    afterUserMove();
    return true;
  }

  function choosePromotion(piece: "q" | "r" | "b" | "n") {
    if (!pendingPromotion) return;
    const ok = applyMove(pendingPromotion + piece);
    setPendingPromotion(null);
    if (ok) afterUserMove();
  }

  // ---------- Screen ----------

  const squareStyles = lastMove ? { [lastMove.from]: LAST_MOVE, [lastMove.to]: LAST_MOVE } : {};
  const status = outcome
    ? outcome.ok
      ? "Well done!"
      : "Let's try again"
    : thinking
      ? "The bot is thinking…"
      : "Your move";

  return (
    <>
    <NavBar
      trail={[
        { label: "Kasparov", onClick: onHome },
        { label: "Endgame Academy", onClick: onBack },
        { label: lesson.title },
      ]}
      escDisabled={!!pendingPromotion}
    />
    <div style={gameLayout}>
      <div style={{ position: "relative", width: "min(480px, 100%)" }}>
        <Chessboard
          options={{
            position: fen,
            onPieceDrop,
            boardOrientation: userSide === "w" ? "white" : "black",
            squareStyles,
            arrows: hint && !outcome ? [hint] : [],
          }}
        />
        {pendingPromotion && (
          <PromotionDialog side={userSide} onChoose={choosePromotion} onCancel={() => setPendingPromotion(null)} />
        )}
      </div>

      <div style={sidePanel}>
        <div>
          <div style={{ fontSize: 13, opacity: 0.6 }}>
            {lesson.level} · Run {run} of 2 {run === 1 ? "(with hints)" : "(on your own)"}
          </div>
          <h2 style={{ margin: "2px 0" }}>{lesson.title}</h2>
          <div style={{ fontSize: 20, fontWeight: 600 }}>{status}</div>
        </div>

        {outcome && (
          <div style={{ ...box, padding: "12px 14px", borderColor: outcome.ok ? "#22c55e" : "#f59e0b" }}>
            <strong>{outcome.text}</strong>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 10 }}>
              {outcome.ok && run === 1 && (
                <button className="btn-primary" onClick={() => start(2)}>
                  Now on your own (run 2) →
                </button>
              )}
              {outcome.ok && run === 2 && onNext && (
                <button className="btn-primary" onClick={onNext}>
                  Next lesson →
                </button>
              )}
              {outcome.ok && run === 2 && <span style={{ alignSelf: "center" }}>Lesson mastered! 🎓</span>}
              <button className={outcome.ok ? undefined : "btn-primary"} onClick={() => start(run)}>
                {outcome.ok ? "Play this run again" : "Try again"}
              </button>
              {!outcome.ok && run === 2 && <button onClick={() => start(1)}>Back to run 1 (with hints)</button>}
            </div>
          </div>
        )}

        <div style={{ ...box, padding: "10px 12px", fontSize: 14, lineHeight: 1.45 }}>
          <p style={{ margin: "0 0 6px" }}>{lesson.intro}</p>
          <strong>{GOAL_TEXT[lesson.goal]}</strong>
          <ul style={{ margin: "6px 0 0", paddingLeft: 20 }}>
            {lesson.tips.map((t) => (
              <li key={t}>{t}</li>
            ))}
          </ul>
        </div>

        <MoveList sans={sans} startsWithBlack={lesson.fen.split(" ")[1] === "b"} maxHeight={120} />
        <CoachPanel notes={notes} maxHeight={260} />
        {!outcome && (
          <button className="btn-block" onClick={() => start(run)}>
            Restart this run
          </button>
        )}
      </div>
    </div>
    </>
  );
}