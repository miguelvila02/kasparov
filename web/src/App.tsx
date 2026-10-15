import { useEffect, useState } from "react";
import CpuGame from "./CpuGame";
import LearnOpenings from "./LearnOpenings";
import EndgameAcademy from "./EndgameAcademy";
import { engineReady, page } from "./shared";
import "./ui.css";

type Mode = "home" | "cpu" | "openings" | "endgames";

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
        <button className="menu-btn" onClick={() => setMode("cpu")}>
          <span className="menu-icon">♚</span>
          <span>
            <strong>User vs CPU</strong>
            <small>Play a full game; the bot explains every move</small>
          </span>
        </button>
        <button className="menu-btn" onClick={() => setMode("openings")}>
          <span className="menu-icon">♞</span>
          <span>
            <strong>Learn Openings</strong>
            <small>Watch an opening move by move, then play it yourself</small>
          </span>
        </button>
        <button className="menu-btn" onClick={() => setMode("endgames")}>
          <span className="menu-icon">♜</span>
          <span>
            <strong>Endgame Academy</strong>
            <small>From easy checkmates to the hardest rook endings</small>
          </span>
        </button>
      </div>
    </div>
  );
}