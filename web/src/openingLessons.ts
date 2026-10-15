// Extra teaching content for "Learn Openings": which side you play, when to choose
// the opening, its main plans and a common pitfall. The move-by-move ideas come from openings.ts.

import type { Side } from "./shared";

export type OpeningLesson = {
  side: Side;
  whenToPlay: string;
  plans: string[];
  pitfall: string;
};

export const LESSONS: Record<string, OpeningLesson> = {
  italian: {
    side: "w",
    whenToPlay:
      "A great first opening for White after 1.e4 e5. Choose it when you want calm, healthy positions with quick development, early castling and plans that are easy to understand.",
    plans: [
      "Develop the knights and bishops quickly and castle early.",
      "Prepare d4 with c3 to build a strong pawn center.",
      "Keep the pressure on f7 and look for Ng5 ideas when Black's kingside gets weak.",
    ],
    pitfall: "Don't start an early Ng5 attack before your pieces are developed: it usually just loses time.",
  },
  ruy: {
    side: "w",
    whenToPlay:
      "For players who like long, strategic games after 1.e4 e5. It is the most respected answer to ...Nc6: the pressure is slow, but it lasts the whole game.",
    plans: [
      "Pressure the e5 pawn by attacking its defender, the knight on c6.",
      "Prepare c3 and d4 to take over the center.",
      "Bring the queen's knight around via d2 and f1 to g3, toward Black's king.",
    ],
    pitfall:
      "Early on, Bxc6 followed by Nxe5 doesn't win a pawn: Black answers ...Qd4 and wins it back. Protect e4 first (Re1).",
  },
  scotch: {
    side: "w",
    whenToPlay:
      "When you want open, active positions and prefer to avoid the long theory of the Ruy Lopez. White opens the center at once with d4.",
    plans: [
      "Trade the d-pawn for Black's e-pawn and use the open d-file.",
      "Use the extra space from the e4 pawn and develop quickly.",
      "In the Mieses line, push e5 to chase Black's pieces and gain space.",
    ],
    pitfall: "The knight on d4 is a target: keep it protected (Be3, c3) before Black attacks it twice.",
  },
  kingsgambit: {
    side: "w",
    whenToPlay:
      "When you want a sharp attacking game and don't mind giving material. Best in fast games, or against opponents who dislike defending.",
    plans: [
      "Open the f-file for your rook after castling.",
      "Win the f4 pawn back later, often with Bxf4.",
      "Attack f7 and the black king quickly.",
    ],
    pitfall: "f4 weakens your own king: watch the e1-h4 diagonal and early queen checks.",
  },
  vienna: {
    side: "w",
    whenToPlay:
      "When you like the ideas of the King's Gambit but want a safer version: Nc3 first protects e4 and prepares f4.",
    plans: [
      "Play f4 to open the f-file and attack.",
      "Develop the bishop to c4, aiming at f7.",
      "Castle kingside and use the f-file for your rook.",
    ],
    pitfall: "After the Vienna Gambit the center opens quickly: finish your development before you attack.",
  },
  queensgambit: {
    side: "w",
    whenToPlay:
      "The classic way to play 1.d4: solid and strategic. It suits players who like steady pressure rather than wild tactics.",
    plans: [
      "Pressure d5 with c4, Nc3 and Bg5.",
      "Use the c-file once the c- and d-pawns are exchanged.",
      "In many lines, advance the queenside pawns (b4–b5) to create weaknesses: the 'minority attack'.",
    ],
    pitfall: "In the Accepted line, don't try to keep the c4 pawn with b3: just develop and win it back with Bxc4.",
  },
  catalan: {
    side: "w",
    whenToPlay:
      "For positional players who like long-term pressure. The bishop on g2 works along the long diagonal for the whole game.",
    plans: [
      "Use the g2 bishop's pressure along the long diagonal toward b7 and a8.",
      "Win back the c4 pawn with Qc2 or Qa4+.",
      "Press on Black's queenside, where your bishop points.",
    ],
    pitfall: "Losing the c4 pawn early is normal here. Don't play passively: win it back with active moves.",
  },
  london: {
    side: "w",
    whenToPlay:
      "When you want one setup you can play against almost anything, with very little theory to learn. Very popular at club level.",
    plans: [
      "Build the pawn pyramid c3–d4–e3 and develop Bf4, Nf3, Nbd2 and Bd3.",
      "Castle kingside, then push e4 when you're ready.",
      "Use the bishop on d3 and your knights for a kingside attack.",
    ],
    pitfall: "Against an early ...c5 and ...Qb6, watch your b2 pawn: you may need Qb3 or Qc1 to protect it.",
  },
  english: {
    side: "w",
    whenToPlay:
      "When you like flexible, positional games and want to avoid the most popular lines. It often turns into other openings.",
    plans: [
      "Control the d5 square with c4 and Nc3.",
      "Fianchetto the bishop on g2 for long-diagonal pressure.",
      "Expand on the queenside with a3, b4 and Rb1.",
    ],
    pitfall: "Don't let Black build a big center for free: meet ...e5 setups with active piece play.",
  },
  reti: {
    side: "w",
    whenToPlay:
      "For players who like to control the center from a distance and keep their options open. A flexible, 'hypermodern' opening.",
    plans: [
      "Attack Black's center from the side with c4, and sometimes e4 later.",
      "Fianchetto both bishops (g3/Bg2 and b3/Bb2).",
      "Wait for Black to commit, then strike in the center.",
    ],
    pitfall: "If Black grabs space with ...d4, challenge that pawn quickly (e3 or b4) before it cramps you.",
  },
  sicilian: {
    side: "b",
    whenToPlay:
      "The most popular reply to 1.e4 for players who want to play for a win as Black. The positions are unbalanced, with chances for both sides.",
    plans: [
      "Use the half-open c-file after ...cxd4.",
      "Expand on the queenside with ...a6 and ...b5.",
      "Strike in the center with ...d5 or ...e5 at the right moment.",
    ],
    pitfall: "White often attacks your king directly: don't delay your development or castling for too long.",
  },
  french: {
    side: "b",
    whenToPlay:
      "A solid reply to 1.e4 for players who like structure and counterattack. Black accepts a slightly cramped position in exchange for a firm center.",
    plans: [
      "Attack the base of White's pawn chain with ...c5.",
      "Add pressure on d4 with ...Nc6 and ...Qb6.",
      "Find a good future for the light-squared bishop, which the e6 pawn blocks.",
    ],
    pitfall: "The bishop on c8 can stay locked in behind the e6 pawn: plan early how you'll activate it.",
  },
  carokann: {
    side: "b",
    whenToPlay:
      "A very solid reply to 1.e4 for players who want a safe position and a healthy pawn structure.",
    plans: [
      "Develop the light-squared bishop before playing ...e6.",
      "Challenge White's center with ...c5 later on.",
      "Aim for a good endgame thanks to your sound pawn structure.",
    ],
    pitfall: "In the Advance Variation, don't let White's extra space suffocate you: hit d4 with ...c5.",
  },
  kingsindian: {
    side: "b",
    whenToPlay:
      "For ambitious players facing 1.d4 who like counterattacks. Black lets White build a big center, then strikes back at it.",
    plans: [
      "Break with ...e5 (or ...c5) against White's center.",
      "When the center is closed, attack on the kingside with ...f5.",
      "Make the most of the strong bishop on g7.",
    ],
    pitfall: "Don't let White's center advance for free: time your ...e5 break well.",
  },
  nimzo: {
    side: "b",
    whenToPlay:
      "A highly respected, flexible reply to 1.d4 with 2.c4 and 3.Nc3: Black controls the e4 square with pieces instead of pawns.",
    plans: [
      "Pin the knight on c3 to stop e4.",
      "Often trade with ...Bxc3 to double White's pawns.",
      "Fight for the light squares (e4 and d5) with ...d5, or with ...b6 and ...Bb7.",
    ],
    pitfall: "Giving up your bishop for the knight is only good if White's pawns become weak: don't trade without a reason.",
  },
};