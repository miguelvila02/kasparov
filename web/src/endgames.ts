// Endgame Academy lessons, from easy to hard.
// Every position was checked with the engine: following its hints reaches the goal,
// with the engine defending as well as it can.

export type Level = "Easy" | "Medium" | "Hard";

/**
 * mate        – checkmate the opponent
 * promote     – make a new queen and keep it
 * winMaterial – leave the opponent with only a king (the rest is a mate you already know)
 */
export type Goal = "mate" | "promote" | "winMaterial";

export type EndgameLesson = {
  id: string;
  level: Level;
  title: string;
  fen: string; // you always play the side to move
  goal: Goal;
  intro: string;
  tips: string[];
};

export const GOAL_TEXT: Record<Goal, string> = {
  mate: "Goal: checkmate.",
  promote: "Goal: make a new queen and keep it.",
  winMaterial: "Goal: win all of the opponent's material, leaving only the king.",
};

export const ENDGAMES: EndgameLesson[] = [
  {
    id: "ladder",
    level: "Easy",
    title: "Ladder mate with two rooks",
    fen: "8/8/8/3k4/8/8/8/R3K2R w - - 0 1",
    goal: "mate",
    intro:
      "Two rooks can checkmate without any help from the king. They take turns: one rook cuts off a rank, the other gives check on the next one, pushing the king to the edge like climbing down a ladder.",
    tips: [
      "One rook cuts the king off; the other gives check.",
      "If the king attacks a rook, move that rook far away along the same line.",
      "Your own king isn't needed here: keep it out of the way.",
    ],
  },
  {
    id: "queen",
    level: "Easy",
    title: "Checkmate with the queen",
    fen: "8/8/4k3/8/8/8/8/3QK3 w - - 0 1",
    goal: "mate",
    intro:
      "The queen can't checkmate on her own: she needs her king. First box the enemy king in with the queen, then bring your king closer and give mate on the edge of the board.",
    tips: [
      "Keep the queen a knight's move away from the enemy king to shrink its box.",
      "Watch out for stalemate: always leave the enemy king a free square until you can mate.",
      "Bring your king up: the mate needs both pieces.",
    ],
  },
  {
    id: "rook",
    level: "Easy",
    title: "Checkmate with a rook",
    fen: "8/8/8/4k3/8/8/8/4K2R w - - 0 1",
    goal: "mate",
    intro:
      "King and rook can mate together. Use the rook as a wall to cut the enemy king off, walk your king toward it, and give check along the edge when the kings stand face to face.",
    tips: [
      "Use the rook like a wall that shrinks the king's area.",
      "Put your king opposite the enemy king, then check with the rook.",
      "If the king attacks your rook, move the rook far away along the same line.",
    ],
  },
  {
    id: "kingpawn",
    level: "Medium",
    title: "King and pawn: king in front",
    fen: "4k3/8/4K3/4P3/8/8/8/8 w - - 0 1",
    goal: "promote",
    intro:
      "Your king stands on the 6th rank in front of its pawn. That is a win no matter whose turn it is. Escort the pawn to the last rank, and don't allow stalemate on the way.",
    tips: [
      "Lead with your king; the pawn follows.",
      "Aim to control the promotion square with your king before the pawn arrives.",
      "Near the end, check that the enemy king still has a legal move: otherwise it's stalemate!",
    ],
  },
  {
    id: "queenpawn",
    level: "Medium",
    title: "Queen against a pawn",
    fen: "8/8/8/5K2/8/8/1pk5/7Q w - - 0 1",
    goal: "winMaterial",
    intro:
      "Black's pawn is one step from becoming a queen, but your queen can stop it. Use checks to force the black king in front of its own pawn, then use that free move to bring your king closer. Repeat until the pawn falls.",
    tips: [
      "Give checks that bring your queen closer to the pawn.",
      "When the king has to stand in front of its pawn, use the free move to bring your king closer.",
      "Watch out for tricks, like promoting to a knight with check.",
    ],
  },
  {
    id: "breakthrough",
    level: "Hard",
    title: "The pawn breakthrough",
    fen: "7k/ppp5/8/PPP5/8/8/8/6K1 w - - 0 1",
    goal: "promote",
    intro:
      "Three pawns against three, and the kings are far away. One pawn sacrifice breaks through: after the right first move, Black can't stop one of your pawns from becoming a queen. Can you find it?",
    tips: [
      "Sacrifice a pawn to open a path. The middle pawn goes first.",
      "Whatever Black captures, push the pawn that has just become free.",
      "Count the moves: the black king is too far away to catch your pawn.",
    ],
  },
  {
    id: "lucena",
    level: "Hard",
    title: "The Lucena position",
    fen: "1K6/1P1k4/8/8/8/8/r7/2R5 w - - 0 1",
    goal: "winMaterial",
    intro:
      "The most important rook endgame. Your pawn is on the 7th rank, your king stands in front of it, and your rook cuts the black king off. The plan is to bring your rook up to shield your king from checks ('building a bridge') and then promote. Black will have to give up the rook for your new queen.",
    tips: [
      "Keep the enemy king cut off with your rook.",
      "Bring your rook to the 4th rank so it can block the checks later: that's the bridge.",
      "Step out from in front of the pawn, then hide from the checks behind your rook.",
    ],
  },
];