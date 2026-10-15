// "Learn Openings": watch an opening move by move, then play it yourself.
import { useEffect, useMemo, useRef, useState } from "react";
import { WasmGame } from "engine";
import { Chessboard } from "react-chessboard";
import { DEFENSES, WHITE_OPENINGS } from "./openings";
import type { BookLine, Opening } from "./openings";
import { LESSONS } from "./openingLessons";
import {
  CoachPanel,
  HINT_COLOR,
  LAST_MOVE,
  MoveList,
  START_FEN,
  box,
  card,
  gameLayout,
  moveLabel,
  page,
  sidePanel,
} from "./shared";
import type { Arrow, Note, NoteKind, Side } from "./shared";

type Mode = "watch" | "try";

/** Wrong tries in a row on the same move before the coach shows the answer. */
const HELP_AFTER = 3;
const AUTOPLAY_MS = 2200;
const OPPONENT_DELAY_MS = 700;

/** Positions along a line: fens[i] = position after i moves. */
function lineFens(line: BookLine): string[] {
  const g = new WasmGame(START_FEN);
  const fens = [g.fen()];
  try {
    for (const m of line.moves) {
      if (!g.play(m.uci)) break;
      fens.push(g.fen());
    }
  } finally {
    g.free();
  }
  return fens;
}

function sideName(s: Side) {
  return s === "w" ? "White" : "Black";
}

// ---------- The list of openings ----------

export default function LearnOpenings({ onExit }: { onExit: () => void }) {
  const [opening, setOpening] = useState<Opening | null>(null);
  if (opening) return <Lesson key={opening.id} opening={opening} onBack={() => setOpening(null)} />;

  const section = (title: string, list: Opening[]) => (
    <>
      <h3 style={{ marginBottom: 8 }}>{title}</h3>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))", gap: 12 }}>
        {list.map((o) => (
          <button key={o.id} style={card} onClick={() => setOpening(o)}>
            <strong>
              {o.name} <span style={{ fontWeight: 400, opacity: 0.6 }}>{o.eco}</span>
            </strong>
            <span style={{ fontFamily: "monospace", fontSize: 13 }}>{o.firstMoves}</span>
            <span style={{ fontSize: 13, opacity: 0.75 }}>{o.description}</span>
          </button>
        ))}
      </div>
    </>
  );

  return (
    <div style={{ ...page, maxWidth: 820, margin: "2.5rem auto" }}>
      <button onClick={onExit}>← Menu</button>
      <h2 style={{ marginBottom: 4 }}>Learn Openings</h2>
      <p style={{ opacity: 0.7, marginTop: 0 }}>
        Pick an opening. First you watch it move by move with the idea behind each move, then you try to play it yourself.
      </p>
      {section("Openings for White", WHITE_OPENINGS)}
      {section("Defenses for Black", DEFENSES)}
    </div>
  );
}

// ---------- One opening: watch and try ----------

function Lesson({ opening, onBack }: { opening: Opening; onBack: () => void }) {
  const lesson = LESSONS[opening.id];
  const side: Side = lesson?.side ?? "w";
  const [lineIdx, setLineIdx] = useState(0);
  const [mode, setMode] = useState<Mode>("watch");
  const line = opening.lines[lineIdx];
  const fens = useMemo(() => lineFens(line), [line]);

  return (
    <div style={gameLayout}>
      {mode === "watch" ? (
        <Watch
          key={`watch-${lineIdx}`}
          opening={opening}
          line={line}
          fens={fens}
          side={side}
          lineIdx={lineIdx}
          setLineIdx={setLineIdx}
          onTry={() => setMode("try")}
          onBack={onBack}
        />
      ) : (
        <Try
          key={`try-${lineIdx}`}
          opening={opening}
          line={line}
          fens={fens}
          side={side}
          onWatch={() => setMode("watch")}
          onBack={onBack}
          onOtherLine={
            opening.lines.length > 1
              ? () => {
                  setLineIdx((i) => (i + 1) % opening.lines.length);
                  setMode("watch");
                }
              : undefined
          }
        />
      )}
    </div>
  );
}

function LineTabs({ opening, lineIdx, setLineIdx }: { opening: Opening; lineIdx: number; setLineIdx: (i: number) => void }) {
  if (opening.lines.length < 2) return null;
  return (
    <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
      {opening.lines.map((l, i) => (
        <button
          key={l.name}
          onClick={() => setLineIdx(i)}
          style={{ fontWeight: i === lineIdx ? 700 : 400, opacity: i === lineIdx ? 1 : 0.75 }}
        >
          {l.name}
        </button>
      ))}
    </div>
  );
}

// ---------- Watch mode ----------

function Watch(props: {
  opening: Opening;
  line: BookLine;
  fens: string[];
  side: Side;
  lineIdx: number;
  setLineIdx: (i: number) => void;
  onTry: () => void;
  onBack: () => void;
}) {
  const { opening, line, fens, side, lineIdx, setLineIdx, onTry, onBack } = props;
  const lesson = LESSONS[opening.id];
  const [step, setStep] = useState(0); // number of moves shown
  const [playing, setPlaying] = useState(false);
  const n = line.moves.length;
  const listRef = useRef<HTMLDivElement>(null);

  // Autoplay
  useEffect(() => {
    if (!playing) return;
    if (step >= n) {
      setPlaying(false);
      return;
    }
    const t = window.setTimeout(() => setStep((s) => Math.min(n, s + 1)), step === 0 ? 400 : AUTOPLAY_MS);
    return () => clearTimeout(t);
  }, [playing, step, n]);

  // Keep the current move visible in the list
  useEffect(() => {
    listRef.current?.querySelector(`[data-step="${step}"]`)?.scrollIntoView({ block: "nearest" });
  }, [step]);

  const last = step > 0 ? line.moves[step - 1] : null;
  const next = step < n ? line.moves[step] : null;
  const squareStyles = last ? { [last.uci.slice(0, 2)]: LAST_MOVE, [last.uci.slice(2, 4)]: LAST_MOVE } : {};
  const arrows: Arrow[] = next ? [{ startSquare: next.uci.slice(0, 2), endSquare: next.uci.slice(2, 4), color: HINT_COLOR }] : [];

  return (
    <>
      <div style={{ width: "min(480px, 100%)" }}>
        <Chessboard
          options={{
            position: fens[step],
            boardOrientation: side === "w" ? "white" : "black",
            squareStyles,
            arrows,
            allowDragging: false,
          }}
        />
        <div style={{ display: "flex", gap: 6, justifyContent: "center", marginTop: 10 }}>
          <button onClick={() => { setPlaying(false); setStep(0); }} disabled={step === 0} title="Start">⏮</button>
          <button onClick={() => { setPlaying(false); setStep((s) => Math.max(0, s - 1)); }} disabled={step === 0} title="Previous move">◀</button>
          <button onClick={() => setPlaying((p) => !p)} disabled={step >= n && !playing} style={{ minWidth: 90 }}>
            {playing ? "⏸ Pause" : "▶ Play"}
          </button>
          <button onClick={() => { setPlaying(false); setStep((s) => Math.min(n, s + 1)); }} disabled={step >= n} title="Next move">▶</button>
          <button onClick={() => { setPlaying(false); setStep(n); }} disabled={step >= n} title="End">⏭</button>
        </div>
      </div>

      <div style={sidePanel}>
        <div>
          <button onClick={onBack}>← All openings</button>
          <h2 style={{ margin: "8px 0 2px" }}>
            {opening.name} <span style={{ fontWeight: 400, opacity: 0.6, fontSize: 16 }}>{opening.eco}</span>
          </h2>
          <div style={{ fontSize: 13, opacity: 0.7 }}>
            You'll play {sideName(side)} · {line.name}
          </div>
        </div>
        <LineTabs opening={opening} lineIdx={lineIdx} setLineIdx={setLineIdx} />

        <div style={{ ...box, padding: "10px 12px", minHeight: 64 }}>
          {last ? (
            <>
              <strong>{moveLabel(step - 1, last.san)}</strong> {last.idea}
            </>
          ) : (
            <span style={{ opacity: 0.75 }}>Press ▶ Play, or step through the moves with ◀ ▶. The purple arrow shows the next move.</span>
          )}
        </div>

        <div ref={listRef} style={{ ...box, maxHeight: 180, fontSize: 13, display: "flex", flexDirection: "column", gap: 4 }}>
          {line.moves.map((m, i) => (
            <div
              key={i}
              data-step={i + 1}
              onClick={() => { setPlaying(false); setStep(i + 1); }}
              style={{ cursor: "pointer", fontWeight: i + 1 === step ? 700 : 400, opacity: i + 1 <= step ? 1 : 0.55 }}
            >
              {moveLabel(i, m.san)}
            </div>
          ))}
        </div>

        {lesson && (
          <div style={{ fontSize: 14, lineHeight: 1.45 }}>
            <strong>When to play it</strong>
            <p style={{ margin: "4px 0 10px" }}>{lesson.whenToPlay}</p>
            <strong>Key plans</strong>
            <ul style={{ margin: "4px 0 10px", paddingLeft: 20 }}>
              {lesson.plans.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
            <strong>Watch out</strong>
            <p style={{ margin: "4px 0 0" }}>{lesson.pitfall}</p>
          </div>
        )}

        <button onClick={onTry} style={{ padding: "10px", fontSize: 16, fontWeight: 600 }}>
          Try it yourself →
        </button>
      </div>
    </>
  );
}

// ---------- Try mode ----------

function Try(props: {
  opening: Opening;
  line: BookLine;
  fens: string[];
  side: Side;
  onWatch: () => void;
  onBack: () => void;
  onOtherLine?: () => void;
}) {
  const { opening, line, fens, side, onWatch, onBack, onOtherLine } = props;
  const n = line.moves.length;
  const [ply, setPly] = useState(0);
  const [showHints, setShowHints] = useState(false);
  const [errors, setErrors] = useState(0); // wrong tries in a row on the current move
  const [revealed, setRevealed] = useState(false); // the coach showed the answer for this move
  const [helped, setHelped] = useState(false); // any help used during this run
  const [notes, setNotes] = useState<Note[]>([]);
  const [run, setRun] = useState(0); // restarts the run when it changes
  const noteId = useRef(0);
  const plyRef = useRef(0);

  const isUserTurn = (p: number) => (p % 2 === 0) === (side === "w");
  const done = ply >= n;

  function addNote(kind: NoteKind, text: string) {
    const id = ++noteId.current;
    setNotes((prev) => [...prev, { id, kind, text }]);
  }

  function advance(p: number) {
    const m = line.moves[p];
    addNote(isUserTurn(p) ? "good" : "book", `${moveLabel(p, m.san)} ${m.idea}`);
    plyRef.current = p + 1;
    setPly(p + 1);
    setErrors(0);
    setRevealed(false);
  }

  /** Starts a run from move 1. `hints` = whether hints are on for this run. */
  function restart(hints: boolean) {
    plyRef.current = 0;
    noteId.current = 0;
    setShowHints(hints);
    setPly(0);
    setErrors(0);
    setRevealed(false);
    setHelped(hints);
    setNotes([
      {
        id: 0,
        kind: "info",
        text: `Play the ${line.name} as ${sideName(side)}. The coach plays the other side's moves. ` +
          `If you're stuck, a wrong move simply goes back, and after ${HELP_AFTER} tries the coach shows you the answer.`,
      },
    ]);
    setRun((r) => r + 1);
  }

  // First run
  useEffect(() => {
    restart(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The opponent's moves are played automatically
  useEffect(() => {
    if (ply >= n || isUserTurn(ply)) return;
    const t = window.setTimeout(() => {
      if (plyRef.current === ply) advance(ply);
    }, OPPONENT_DELAY_MS);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ply, n, run]);

  // Hints count as help for this run
  useEffect(() => {
    if (showHints) setHelped(true);
  }, [showHints]);

  function onPieceDrop({ sourceSquare, targetSquare }: { sourceSquare: string; targetSquare: string | null }) {
    if (!targetSquare || done || !isUserTurn(ply)) return false;
    const uci = sourceSquare + targetSquare;
    const expected = line.moves[ply];
    if (uci === expected.uci) {
      advance(ply);
      return true;
    }

    // Is it at least a legal move? (Illegal drops just snap back.)
    const g = new WasmGame(fens[ply]);
    let san: string | undefined;
    try {
      san = g.legal_moves().includes(uci) ? g.san(uci) : undefined;
    } finally {
      g.free();
    }
    if (!san) return false;

    const tries = errors + 1;
    setErrors(tries);
    if (tries >= HELP_AFTER && !revealed) {
      setRevealed(true);
      setHelped(true);
      addNote("hint", `Here's the move: ${moveLabel(ply, expected.san)}. ${expected.idea} Now play it on the board.`);
    } else if (!revealed) {
      const left = HELP_AFTER - tries;
      addNote("warn", `${san} isn't the move in this line. Let's try again! (${left} more tr${left === 1 ? "y" : "ies"} before the coach helps)`);
    } else {
      addNote("warn", `Not quite: the arrow shows ${expected.san}.`);
    }
    return false; // the piece goes back
  }

  const expected = !done ? line.moves[ply] : null;
  const showArrow = expected && isUserTurn(ply) && (showHints || revealed);
  const arrows: Arrow[] = showArrow
    ? [{ startSquare: expected.uci.slice(0, 2), endSquare: expected.uci.slice(2, 4), color: HINT_COLOR }]
    : [];
  const last = ply > 0 ? line.moves[ply - 1] : null;
  const squareStyles = last ? { [last.uci.slice(0, 2)]: LAST_MOVE, [last.uci.slice(2, 4)]: LAST_MOVE } : {};
  const status = done
    ? "Done!"
    : isUserTurn(ply)
      ? `Your move (${sideName(side)})`
      : "The coach is playing the other side…";

  return (
    <>
      <div style={{ width: "min(480px, 100%)" }}>
        <Chessboard
          options={{
            position: fens[ply],
            onPieceDrop,
            boardOrientation: side === "w" ? "white" : "black",
            squareStyles,
            arrows,
          }}
        />
      </div>

      <div style={sidePanel}>
        <div>
          <button onClick={onWatch}>← Watch again</button>
          <h2 style={{ margin: "8px 0 2px" }}>{opening.name}</h2>
          <div style={{ fontSize: 13, opacity: 0.7 }}>
            Try it yourself · {line.name} · move {Math.floor(Math.min(ply, n - 1) / 2) + 1} of {Math.ceil(n / 2)}
          </div>
          <div style={{ fontSize: 20, fontWeight: 600, marginTop: 6 }}>{status}</div>
        </div>

        <label style={{ fontSize: 14, display: "flex", gap: 6, alignItems: "center", cursor: "pointer" }}>
          <input type="checkbox" checked={showHints} onChange={(e) => setShowHints(e.target.checked)} disabled={done} />
          Show hints (an arrow shows the move to play)
        </label>

        {done && (
          <div style={{ ...box, padding: "12px 14px", borderColor: helped ? "#f59e0b" : "#22c55e" }}>
            {helped ? (
              <>
                <strong>You reached the final position of the {line.name}, with some help.</strong>
                <p style={{ margin: "6px 0 10px" }}>
                  Now try the whole opening again on your own, without hints, to make sure you really know it.
                </p>
                <button onClick={() => restart(false)} style={{ fontWeight: 600 }}>
                  Try again on my own
                </button>
              </>
            ) : (
              <>
                <strong>Perfect! You played the whole {line.name} on your own.</strong>
                <p style={{ margin: "6px 0 10px" }}>You know this line. Try another line or another opening.</p>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <button onClick={() => restart(showHints)}>Play it again</button>
                  {onOtherLine && <button onClick={onOtherLine}>Next line</button>}
                  <button onClick={onBack}>Other openings</button>
                </div>
              </>
            )}
          </div>
        )}

        <MoveList sans={line.moves.slice(0, ply).map((m) => m.san)} maxHeight={140} />
        <CoachPanel notes={notes} maxHeight={260} />
        {!done && (
          <button onClick={() => restart(showHints)}>Restart from move 1</button>
        )}
      </div>
    </>
  );
}