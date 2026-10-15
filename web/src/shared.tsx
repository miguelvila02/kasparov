// Pieces shared by the three modes: constants, helpers and small UI components.
import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import init from "engine";

/** Loads the engine once (the page's copy is used for the rules; thinking happens in a worker). */
export const engineReady = init();

export const START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

export type Side = "w" | "b";
export type NoteKind = "book" | "info" | "warn" | "engine" | "hint" | "good" | "inaccuracy" | "mistake" | "blunder";
/** `ply` = the move the note belongs to (0 = White's first move); notes without it are general. */
export type Note = { id: number; kind: NoteKind; text: string; ply?: number };
export type Arrow = { startSquare: string; endSquare: string; color: string };

export function createWorker() {
  return new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" });
}

export function formatScore(cp: number) {
  if (Math.abs(cp) > 29000) {
    const moves = Math.ceil((30000 - Math.abs(cp)) / 2);
    return `${cp > 0 ? "White" : "Black"} mates in ${moves}`;
  }
  return `${cp >= 0 ? "+" : ""}${(cp / 100).toFixed(2)}`;
}

/** Game-over text from the user's point of view, or null while the game goes on. */
export function resultText(status: string, user: Side): string | null {
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
export function moveLabel(ply: number, san: string, firstPlyIsBlack = false) {
  const p = ply + (firstPlyIsBlack ? 1 : 0);
  const n = Math.floor(p / 2) + 1;
  return p % 2 === 0 ? `${n}.${san}` : `${n}...${san}`;
}

export const LAST_MOVE: CSSProperties = { background: "rgba(255, 213, 79, 0.45)" };

export const NOTE_COLORS: Record<NoteKind, string> = {
  book: "#3b82f6",
  info: "#9ca3af",
  warn: "#f59e0b",
  engine: "#10b981",
  hint: "#8b5cf6",
  good: "#22c55e",
  inaccuracy: "#eab308",
  mistake: "#f97316",
  blunder: "#ef4444",
};

// Plan arrows: a move, the reply that's expected, and the follow-up
export const ARROW_COLORS = ["rgba(37, 99, 235, 0.85)", "rgba(234, 88, 12, 0.75)", "rgba(37, 99, 235, 0.4)"];
export const HINT_COLOR = "rgba(139, 92, 246, 0.85)";

export const page: CSSProperties = { fontFamily: "sans-serif", padding: "0 16px" };

export const box: CSSProperties = {
  border: "1px solid rgba(128, 128, 128, 0.35)",
  borderRadius: 8,
  padding: "6px 10px",
  overflowY: "auto",
};

/** One step in the navigation path. Steps with `onClick` can be clicked to go back there. */
export type Crumb = { label: string; onClick?: () => void };

/**
 * The bar at the top of every screen: "← Back" (to the previous step), the clickable
 * path (Kasparov › Learn Openings › Italian Game), and Esc as a shortcut for Back.
 */
export function NavBar({ trail, escDisabled = false }: { trail: Crumb[]; escDisabled?: boolean }) {
  const back = trail
    .slice(0, -1)
    .reverse()
    .find((c) => c.onClick);

  useEffect(() => {
    if (!back?.onClick || escDisabled) return;
    const go = back.onClick;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") go();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [back, escDisabled]);

  return (
    <nav className="navbar" aria-label="Navigation">
      {back && (
        <button className="btn-ghost" onClick={back.onClick} title={`Back to ${back.label} (Esc)`}>
          ← Back
        </button>
      )}
      <ol className="crumbs">
        {trail.map((c, i) => {
          const last = i === trail.length - 1;
          return (
            <li key={i}>
              {c.onClick && !last ? (
                <button className="crumb-link" onClick={c.onClick}>
                  {c.label}
                </button>
              ) : (
                <span aria-current={last ? "page" : undefined}>{c.label}</span>
              )}
            </li>
          );
        })}
      </ol>
      {back && <span className="nav-hint">Esc = back</span>}
    </nav>
  );
}

/** Board + side panel layout used by every mode. */
export const gameLayout: CSSProperties = {
  ...page,
  display: "flex",
  flexWrap: "wrap",
  gap: 24,
  justifyContent: "center",
  alignItems: "flex-start",
  margin: "2rem auto",
  maxWidth: 900,
};

export const sidePanel: CSSProperties = {
  width: 320,
  maxWidth: "100%",
  display: "flex",
  flexDirection: "column",
  gap: 12,
  textAlign: "left",
};

/** The scrolling list of coach notes. */
export function CoachPanel({ notes, title = "Coach", maxHeight = 300 }: { notes: Note[]; title?: string; maxHeight?: number }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.scrollTo({ top: ref.current.scrollHeight });
  }, [notes]);
  return (
    <div>
      <div style={{ fontSize: 13, opacity: 0.6, marginBottom: 4 }}>{title}</div>
      <div ref={ref} style={{ ...box, maxHeight, display: "flex", flexDirection: "column", gap: 8 }}>
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
  );
}

/** Two-column move list. `startsWithBlack` for positions where Black moves first. */
export function MoveList({ sans, marks = {}, startsWithBlack = false, maxHeight = 200 }: {
  sans: string[];
  marks?: Record<number, string>;
  startsWithBlack?: boolean;
  maxHeight?: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.scrollTo({ top: ref.current.scrollHeight });
  }, [sans]);
  const cells = (startsWithBlack ? ["…"] : []).concat(sans.map((s, i) => s + (marks[i] ?? "")));
  const rows: [number, string, string | undefined][] = [];
  for (let i = 0; i < cells.length; i += 2) rows.push([i / 2 + 1, cells[i], cells[i + 1]]);
  return (
    <div ref={ref} style={{ ...box, maxHeight, fontFamily: "monospace", fontSize: 15 }}>
      {rows.length === 0 && <div style={{ opacity: 0.5 }}>No moves yet</div>}
      {rows.map(([n, white, black]) => (
        <div key={n} style={{ display: "grid", gridTemplateColumns: "36px 1fr 1fr", padding: "2px 0" }}>
          <span style={{ opacity: 0.5 }}>{n}.</span>
          <span>{white}</span>
          <span>{black ?? ""}</span>
        </div>
      ))}
    </div>
  );
}

/** Overlay that asks which piece to promote to. */
export function PromotionDialog({ side, onChoose, onCancel }: {
  side: Side;
  onChoose: (piece: "q" | "r" | "b" | "n") => void;
  onCancel: () => void;
}) {
  const pieces =
    side === "w"
      ? ([["q", "♕", "Queen"], ["r", "♖", "Rook"], ["b", "♗", "Bishop"], ["n", "♘", "Knight"]] as const)
      : ([["q", "♛", "Queen"], ["r", "♜", "Rook"], ["b", "♝", "Bishop"], ["n", "♞", "Knight"]] as const);
  return (
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
        {pieces.map(([letter, symbol, name]) => (
          <button
            key={letter}
            title={name}
            onClick={() => onChoose(letter)}
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
      <button onClick={onCancel}>Cancel</button>
    </div>
  );
}