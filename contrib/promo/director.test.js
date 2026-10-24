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

test("the camera holds the whole window until the hold is over", () => {
  const timed = [{ kind: "click", at: 1, rect: [1000, 600, 1100, 640] }];
  const { camera, moves } = film(timed, 10, 4, 1.5);
  const held = camera.slice(0, Math.round(1.5 * FPS));
  for (const [i, c] of held.entries()) {
    assert.ok(Math.abs(c.zoom - 1) < 1e-9 && c.x === 600 && c.y === 375, `frame ${i}: ${JSON.stringify(c)}`);
  }
  assert.ok(moves.every(m => m.at >= 1.5), "a move was recorded in the hold");
});

test("the wide shot is held until half a beat after the intro", () => {
  const timeline = schedule(record([{ scene: 0, frame: 1, kind: "click", rect: [1000, 600, 1100, 640] }]));
  const until = timeline.intro.to + timeline.beat / 2;
  const [click] = place(record([{ scene: 0, frame: 1, kind: "click", rect: [1000, 600, 1100, 640] }])).timed;
  // The click is near enough that, held or not, the camera would be on its way.
  assert.ok(click.at - 0.85 < until, `the click at ${click.at} s is not early`);
  for (let i = 0; i < Math.round(until * FPS); i++) {
    assert.ok(Math.abs(timeline.camera[i].zoom - 1) < 1e-9, `zoom ${timeline.camera[i].zoom} at ${i / FPS} s`);
  }
  assert.ok(timeline.camera[Math.round((until + 1) * FPS)].zoom > 1.2, "the camera goes in after the hold");
});

test("the spotlight follows the action while the camera rests", () => {
  const column = [[300, 100, 400, 120], [300, 140, 400, 160], [300, 180, 400, 200]];
  // The pointer comes first, so the spot is on the column when the keys start.
  const events = [
    { scene: 0, frame: 1, kind: "point", rect: column[0] },
    { scene: 0, frame: 1, kind: "hold", seconds: 1 },
  ];
  column.forEach((rect, i) => {
    events.push({ scene: 0, frame: 2 + i, kind: "command", keys: "↓", rect });
    events.push({ scene: 0, frame: 2 + i, kind: "hold", seconds: 1 });
  });
  const lit = record(events, [{ caption: "Move down the rows." }]);
  const timeline = schedule(lit);
  const commands = place(lit).timed.filter(e => e.kind === "command");
  assert.equal(commands.length, 3);
  commands.forEach((command, i) => {
    const spot = timeline.spot[Math.round((command.at + 0.6) * FPS)];
    const middle = (spot.y0 + spot.y1) / 2, row = (column[i][1] + column[i][3]) / 2;
    assert.ok(Math.abs(middle - row) < 3, `row ${i}: the spot is at ${middle}, the row at ${row}`);
    assert.ok(spot.alpha > 0.99, `row ${i}: alpha ${spot.alpha}`);
  });
});

test("the pointer stays hidden while the keys go, from the first", () => {
  const events = [
    { scene: 0, frame: 1, kind: "point", rect: [300, 96, 520, 120] },
    { scene: 0, frame: 2, kind: "click", rect: [300, 96, 520, 120] },
    ...typing("select title from books", 3),
  ];
  const timeline = schedule(record(events));
  const keys = place(record(events)).timed.filter(e => e.kind === "type");
  const from = Math.ceil((keys[0].at + 0.15) * FPS), to = Math.floor(keys[keys.length - 1].at * FPS);
  for (let i = from; i <= to; i++) {
    assert.equal(timeline.pointer[i].shown, 0, `the pointer shows at ${i / FPS} s`);
  }
});

test("when the action stops, the camera and the spotlight let go", () => {
  const timed = [{ kind: "click", at: 1, rect: [1000, 600, 1100, 640] }];
  const { camera, spot } = film(timed, 10, 6);
  assert.ok(camera[Math.round(1.6 * FPS)].zoom > 1.3, "the camera went in for the click");
  // Back to the whole window, drifting in a little.
  const after = Math.round(4 * FPS);
  assert.ok(camera[after].zoom < 1.05 && camera[after].zoom > 1, `zoom ${camera[after].zoom}`);
  assert.ok(Math.abs(camera[after].x - 600) < 2 && Math.abs(camera[after].y - 375) < 2, JSON.stringify(camera[after]));
  assert.ok(spot[after].alpha < 0.01, `the spotlight is still on: ${spot[after].alpha}`);
});

test("the camera stays in when the next step comes soon", () => {
  const timed = [
    { kind: "click", at: 1, rect: [1000, 600, 1100, 640] },
    { kind: "click", at: 3, rect: [1000, 640, 1100, 680] },
  ];
  const { camera } = film(timed, 10, 4);
  for (let i = Math.round(1.6 * FPS); i < Math.round(3 * FPS); i++) {
    assert.ok(camera[i].zoom > 1.3, `zoom ${camera[i].zoom} at ${i / FPS} s`);
  }
});

test("typing in a wide field is framed at the field's left", () => {
  const editor = [305, 146, 1200, 398];
  const timed = [0, 1, 2, 3].map(i => ({ kind: "type", text: "s", at: 1 + i / 13, rect: editor }));
  const { camera } = film(timed, 10, 2);
  const now = camera[Math.round(1.25 * FPS)];
  assert.ok(now.zoom > 1.15, `zoom ${now.zoom}`);
  assert.ok(now.x < 600, `the camera looks at x ${now.x}, right of the window's middle`);
});

test("a key after typing waits a moment", () => {
  const events = [...typing("the"), { scene: 0, frame: 4, kind: "press", keys: "↩" }];
  const timed = place(record(events)).timed;
  const [last, enter] = timed.slice(-2);
  assert.equal(enter.kind, "press");
  assert.ok(enter.at - last.at >= 0.25, `${enter.at - last.at} s`);
});

test("the spring never overshoots", () => {
  const timed = [{ kind: "click", at: 1, rect: [1000, 600, 1100, 640] }];
  // One move: filmed until the action stops and the camera pulls back.
  const { camera } = film(timed, 10, 1.9);
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
