// The director's tests: node --test contrib/promo/director.test.js

"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { schedule, place, film, FPS } = require("./director.js");

// A record as the runner writes one.
const record = (events, scenes = [{ caption: "Open a table." }], tempo = 120) => ({
  title: "Test",
  tempo,
  size: [1200, 750],
  scale: 2,
  scenes,
  events: [{ scene: null, frame: 0, kind: "start" }, ...events],
});

const rect = [100, 100, 200, 120];
const typing = (text, from = 1) =>
  [...text].map((character, i) => ({
    scene: 0, frame: from + i, kind: "type", text: character,
    rect: [300 + 8 * i, 96, 308 + 8 * i, 116],
  }));

test("a scene starts after the intro's bar and ends on a bar", () => {
  const { scenes, bar } = place(record([{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }]));
  assert.equal(bar, 2);
  assert.equal(scenes[0].from, 2);
  assert.equal(scenes[0].to % bar, 0);
  assert.ok(scenes[0].to > scenes[0].from);
});

test("a caption gets the time to be read", () => {
  const caption = "one two three four five six seven eight nine ten";
  const { scenes } = place(record([{ scene: 0, frame: 1, kind: "press", keys: "↩", rect }], [{ caption }]));
  assert.ok(scenes[0].to - scenes[0].from >= 3, `${scenes[0].to - scenes[0].from} s`);
});

test("typing is human, and the same every time", () => {
  const events = typing("select title from books");
  const first = schedule(record(events));
  assert.deepEqual(schedule(record(events)), first);
  const times = place(record(events)).timed.map(e => e.at);
  const gaps = times.slice(1).map((t, i) => t - times[i]);
  assert.ok(new Set(gaps.map(g => g.toFixed(3))).size > 5, "the gaps differ");
  const rate = (times.length - 1) / (times[times.length - 1] - times[0]);
  assert.ok(rate > 9 && rate < 17, `${rate} characters a second`);
});

test("the pointer arrives when its point is due, and the click follows", () => {
  const timeline = schedule(record([
    { scene: 0, frame: 1, kind: "point", rect: [100, 200, 300, 220] },
    { scene: 0, frame: 2, kind: "click", rect: [100, 200, 300, 220] },
  ]));
  const [point] = place(record([{ scene: 0, frame: 1, kind: "point", rect: [100, 200, 300, 220] }])).timed;
  const arrived = timeline.pointer[Math.ceil(point.at * FPS)];
  assert.ok(Math.abs(arrived.x - 200) < 1 && Math.abs(arrived.y - 210) < 1, JSON.stringify(arrived));
  assert.equal(timeline.clicks.length, 1);
  assert.ok(timeline.clicks[0].at > point.at);
});

test("the camera holds still while typing goes along a line", () => {
  const { timed, outro } = place(record(typing("select title from books")));
  const { moves } = film(timed, outro, outro + 1);
  // One move to the line, one back to the whole window for the outro.
  assert.ok(moves.length <= 3, `${moves.length} moves`);
});

test("the spring never overshoots", () => {
  const timed = [{ kind: "click", at: 1, rect: [1000, 600, 1100, 640] }];
  const { camera } = film(timed, 10, 4);
  const xs = camera.map(c => c.x);
  for (let i = 1; i < xs.length; i++) assert.ok(xs[i] >= xs[i - 1] - 1e-9, `x went back at frame ${i}`);
  const target = camera[camera.length - 1].x;
  assert.ok(xs.every(x => x <= target + 1e-6), "x went past where it stopped");
});

test("the camera shows the whole window for the outro", () => {
  const timeline = schedule(record([{ scene: 0, frame: 1, kind: "click", rect: [1000, 600, 1100, 640] }]));
  const last = timeline.camera[timeline.camera.length - 1];
  assert.ok(Math.abs(last.zoom - 1) < 0.01 && Math.abs(last.x - 600) < 2 && Math.abs(last.y - 375) < 2, JSON.stringify(last));
});

test("every frame shown is one the runner wrote, in order", () => {
  const timeline = schedule(record([...typing("abc"), { scene: 0, frame: 3, kind: "hold", seconds: 1 }]));
  assert.deepEqual(timeline.frames.map(f => f[1]), [0, 1, 2, 3]);
  const at = timeline.frames.map(f => f[0]);
  assert.deepEqual(at, [...at].sort((a, b) => a - b));
});

test("a command shows its keycap, and makes its sounds", () => {
  const timeline = schedule(record([{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }]));
  assert.equal(timeline.keycaps.length, 1);
  assert.equal(timeline.keycaps[0].keys, "⌘F");
  const kinds = timeline.sounds.map(s => s.kind);
  assert.ok(kinds.includes("key") && kinds.includes("tick") && kinds.includes("bell"), kinds.join());
});

test("an event the director does not know stops it", () => {
  assert.throws(() => place(record([{ scene: 0, frame: 1, kind: "teleport" }])), /teleport/);
});
