import assert from "node:assert/strict";
import { WHITE_OPENINGS } from "../web/src/openings";
import { bookLines, isInBook, lastBookMove, nextBookMoves, openingName, pickBookMove } from "../web/src/book";

const italian = WHITE_OPENINGS.find((o) => o.id === "italian")!;
const london = WHITE_OPENINGS.find((o) => o.id === "london")!;

// Bot as White in the Italian: first move is e4
assert.deepEqual(nextBookMoves(bookLines(italian), []).map((m) => m.uci), ["e2e4"]);
// After 1.e4 e5 2.Nf3 Nc6, White's book move is Bc4
assert.deepEqual(nextBookMoves(bookLines(italian), ["e2e4", "e7e5", "g1f3", "b8c6"]).map((m) => m.san), ["Bc4"]);
// Black has two book replies after 3.Bc4
const black = nextBookMoves(bookLines(italian), ["e2e4", "e7e5", "g1f3", "b8c6", "f1c4"]).map((m) => m.san).sort();
assert.deepEqual(black, ["Bc5", "Nf6"]);
// 1...c5 (Sicilian) leaves the Italian
assert.equal(isInBook(bookLines(italian), ["e2e4", "c7c5"]), false);
// London: against 1...Nf6 the bot still plays Bf4
assert.deepEqual(nextBookMoves(bookLines(london), ["d2d4", "g8f6"]).map((m) => m.san), ["Bf4"]);
// "No preference": first move varies between e4, d4, c4 and Nf3
const firsts = new Set(nextBookMoves(bookLines(null), []).map((m) => m.san));
assert.deepEqual([...firsts].sort(), ["Nf3", "c4", "d4", "e4"]);
// Picking always returns a legal option from the list
assert.ok(pickBookMove(bookLines(null), ["e2e4"], () => 0.99));
// At the end of a line there are no more book moves, but the line is still "in book"
const line = italian.lines[0].moves.map((m) => m.uci);
assert.equal(nextBookMoves(bookLines(italian), line).length, 0);
assert.equal(isInBook(bookLines(italian), line), true);
// Names
assert.equal(openingName("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3"), "Italian Game (C50)");
assert.equal(openingName("8/8/8/8/8/8/8/k6K w - - 0 1"), null);

assert.equal(lastBookMove(bookLines(italian), ["e2e4", "e7e5"])?.san, "e5");
assert.equal(lastBookMove(bookLines(italian), ["e2e4", "c7c5"]), null);
console.log("book tests passed");