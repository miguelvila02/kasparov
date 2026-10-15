import { useEffect, useState } from "react";
import type { CSSProperties } from "react";
import CpuGame from "./CpuGame";
import LearnOpenings from "./LearnOpenings";
import EndgameAcademy from "./EndgameAcademy";
import { engineReady, page } from "./shared";

type Mode = "home" | "cpu" | "openings" | "endgames";

const menuButton: CSSProperties = {
  width: "100%",
  padding: "16px 20px",
  fontSize: 18,
  borderRadius: 12,
  border: "2px solid #b58863",
  background: "#f0d9b5",
  color: "#222",
  cursor: "pointer",
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap: 2,
};

export default function App() {
  const [ready, setReady] = useState(false);
  const [mode, setMode] = useState<Mode>("home");

  useEffect(() => {
    engineReady.then(() => setReady(true));
  }, []);

  if (!ready) return <p style={{ ...page, textAlign: "center", marginTop: "4rem" }}>Loading engine…</p>;

  const home = () => setMode("home");
  if (mode === "cpu") return <CpuGame onExit={home} />;
  if (mode === "openings") return <LearnOpenings onExit={home} />;
  if (mode === "endgames") return <EndgameAcademy onExit={home} />;

  return (
    <div style={{ ...page, maxWidth: 420, margin: "4rem auto", textAlign: "center" }}>
      <h1 style={{ fontSize: 44, marginBottom: 6 }}>Kasparov</h1>
      <p style={{ opacity: 0.7, marginTop: 0, marginBottom: 32 }}>Play, learn openings, master endgames.</p>
      <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
        <button style={menuButton} onClick={() => setMode("cpu")}>
          <strong>User vs CPU</strong>
          <span style={{ fontSize: 13, opacity: 0.75 }}>Play a full game; the bot explains every move</span>
        </button>
        <button style={menuButton} onClick={() => setMode("openings")}>
          <strong>Learn Openings</strong>
          <span style={{ fontSize: 13, opacity: 0.75 }}>Watch an opening move by move, then play it yourself</span>
        </button>
        <button style={menuButton} onClick={() => setMode("endgames")}>
          <strong>Endgame Academy</strong>
          <span style={{ fontSize: 13, opacity: 0.75 }}>From easy checkmates to the hardest rook endings</span>
        </button>
      </div>
    </div>
  );
}