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

test("a scene starts after the intro's bar and ends on a half bar", () => {
  const { scenes, bar } = place(record([{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }]));
  assert.equal(bar, 2);
  assert.equal(scenes[0].from, 2);
  assert.equal(scenes[0].to % (bar / 2), 0);
  assert.ok(scenes[0].to > scenes[0].from);
});

test("a scene ends soon after its last step", () => {
  for (let n = 1; n <= 12; n++) {
    const { scenes, timed, bar } = place(record(
      [{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }, ...typing("select title".slice(0, n), 2)],
      [{ caption: null }],
    ));
    const tail = scenes[0].to - timed[timed.length - 1].at;
    assert.ok(tail <= 0.4 + bar / 2 + 1e-9, `${n} typed: ${tail.toFixed(2)} s after the last step`);
  }
});

test("the outro is a bar and a half", () => {
  const timeline = schedule(record([{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }]));
  assert.ok(Math.abs(timeline.duration - timeline.outro - 1.5 * timeline.bar) < 1e-9);
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
  assert.ok(rate > 14 && rate < 22, `${rate} characters a second`);
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
  // The zoom goes no further than 1.25, and the camera is soon back out.
  const after = timeline.camera.slice(Math.round(until * FPS), Math.round((until + 1) * FPS));
  assert.ok(Math.max(...after.map(c => c.zoom)) > 1.2, "the camera goes in after the hold");
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
  assert.ok(camera[Math.round(1.6 * FPS)].zoom > 1.2, "the camera went in for the click");
  // Back to the whole window, drifting in a little.
  const after = Math.round(4 * FPS);
  assert.ok(camera[after].zoom < 1.05 && camera[after].zoom > 1, `zoom ${camera[after].zoom}`);
  assert.ok(Math.abs(camera[after].x - 600) < 2 && Math.abs(camera[after].y - 375) < 2, JSON.stringify(camera[after]));
  assert.ok(spot[after].alpha < 0.01, `the spotlight is still on: ${spot[after].alpha}`);
});

test("the camera stays in when the next step comes soon", () => {
  const timed = [
    { kind: "click", at: 1, rect: [1000, 600, 1100, 640] },
    { kind: "click", at: 2.4, rect: [1000, 640, 1100, 680] },
  ];
  const { camera } = film(timed, 10, 4);
  for (let i = Math.round(1.6 * FPS); i < Math.round(2.4 * FPS); i++) {
    assert.ok(camera[i].zoom > 1.2, `zoom ${camera[i].zoom} at ${i / FPS} s`);
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
  const { camera } = film(timed, 10, 1.6);
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

test("the camera's moves make no sound", () => {
  const events = [
    { scene: 0, frame: 1, kind: "click", rect: [1000, 600, 1100, 640] },
    { scene: 0, frame: 2, kind: "hold", seconds: 3 },
    { scene: 0, frame: 2, kind: "click", rect: [20, 100, 120, 120] },
  ];
  const { timed, outro } = place(record(events));
  assert.ok(film(timed, outro, outro + 1).moves.length > 1, "the camera moved");
  const kinds = new Set(schedule(record(events)).sounds.map(s => s.kind));
  assert.deepEqual([...kinds].sort(), ["bell", "click"]);
});

// A slide as the runner writes one: no caption, a headline and its line.
const slide = (title, text) => ({ caption: null, slide: title, text });

test("a slide lasts long enough to read, ends on a half bar, and plays nothing", () => {
  const text = "Every command has a key, and the keys are yours.";
  const { scenes, timed, bar } = place(record(
    [{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }],
    [{ caption: "Open a table." }, slide("Keyboard first.", text)],
  ));
  const card = scenes[1];
  assert.deepEqual(card.slide, { title: "Keyboard first.", text });
  assert.equal(card.caption, null);
  const long = card.to - card.from;
  assert.ok(long >= 0.22 * 12 + 0.9 && long < 0.22 * 12 + 0.9 + bar / 2, `${long.toFixed(2)} s for 12 words`);
  assert.equal(card.to % (bar / 2), 0);
  assert.ok(timed.every(e => e.scene !== 1));
});

test("the camera shows the whole window for a slide, and a bell marks it", () => {
  const corner = [1000, 600, 1100, 640];
  const timeline = schedule(record(
    [{ scene: 0, frame: 1, kind: "click", rect: corner }, { scene: 2, frame: 2, kind: "click", rect: corner }],
    [{ caption: null }, slide("Small."), { caption: null }],
  ));
  const card = timeline.scenes[1];
  const mid = timeline.camera[Math.round(((card.from + card.to) / 2) * FPS)];
  assert.ok(Math.abs(mid.zoom - 1) < 0.02, `zoom ${mid.zoom} in the slide`);
  assert.ok(timeline.sounds.some(s => s.kind === "bell" && Math.abs(s.at - card.from) < 1e-9));
});

test("an event the director does not know stops it", () => {
  assert.throws(() => place(record([{ scene: 0, frame: 1, kind: "teleport" }])), /teleport/);
});

// A line the voice says, as render.cjs measures it.
const said = (say, seconds, file = "file:///voice/01.wav") => ({ say, speech: { file, seconds } });

test("a long line lengthens its scene, with room after it, on a half bar", () => {
  const events = [{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }];
  const quiet = place(record(events)).scenes[0];
  const { scenes, bar } = place(record(events, [{ caption: null, ...said("A long line.", 6) }]));
  const scene = scenes[0];
  assert.ok(scene.to - scene.from > quiet.to - quiet.from, "the scene is no longer");
  assert.ok(scene.to >= scene.from + 0.2 + 6 + 0.4 - 1e-9, `${scene.to - scene.from} s`);
  assert.ok(scene.to < scene.from + 0.2 + 6 + 0.4 + bar / 2, `${scene.to - scene.from} s`);
  assert.equal(scene.to % (bar / 2), 0);
});

test("a short line changes nothing", () => {
  const events = [
    { scene: 0, frame: 1, kind: "command", keys: "⌘F", rect },
    { scene: 0, frame: 1, kind: "hold", seconds: 3 },
  ];
  const quiet = place(record(events, [{ caption: null }]));
  const spoken = place(record(events, [{ caption: null, ...said("Short.", 0.8) }]));
  assert.deepEqual(spoken.scenes.map(s => [s.from, s.to]), quiet.scenes.map(s => [s.from, s.to]));
  assert.deepEqual(spoken.timed, quiet.timed);
});

test("a line starts 0.2 s into a shot, and a slide's once its headline is up", () => {
  const timeline = schedule(record(
    [{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }],
    [
      { caption: null, ...said("Open a table.", 1, "file:///v/01.wav") },
      { ...slide("Keyboard first.", null), ...said("Every command has a key.", 1.5, "file:///v/02.wav") },
    ],
  ));
  const [shot, card] = timeline.scenes;
  assert.equal(timeline.voices.length, 2);
  assert.deepEqual(timeline.voices[0], { at: shot.from + 0.2, file: "file:///v/01.wav", seconds: 1, text: "Open a table." });
  const up = timeline.voices[1].at - card.from;
  assert.ok(up >= 0.5 && up <= 1.2, `the line starts ${up} s into the slide`);
  assert.equal(timeline.voices[1].text, "Every command has a key.");
});

test("the steps and the line run at the same time", () => {
  const events = [{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }];
  const quiet = place(record(events)).timed;
  const spoken = place(record(events, [{ caption: null, ...said("A line.", 5) }])).timed;
  assert.deepEqual(spoken, quiet);
});

test("the intro and the outro may have a line, and wait for it", () => {
  const events = [{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }];
  const plain = schedule(record(events));
  const spoken = schedule({
    ...record(events),
    intro: said("Here is what is new in Tabletist. Let's jump in.", 3, "file:///v/00.wav"),
    outro: said("Tabletist is free. Download it today.", 4, "file:///v/02.wav"),
  });
  const [intro, outro] = [spoken.voices[0], spoken.voices[spoken.voices.length - 1]];
  assert.equal(intro.file, "file:///v/00.wav");
  assert.ok(intro.at > 0 && intro.at < 1, `the intro's line at ${intro.at}`);
  assert.ok(spoken.intro.to >= intro.at + 3 + 0.4, `the intro ends at ${spoken.intro.to}`);
  assert.equal(spoken.intro.to % (spoken.bar / 2), 0);
  assert.equal(spoken.scenes[0].from, spoken.intro.to);
  assert.ok(outro.at >= spoken.outro, "the outro's line starts with the outro");
  assert.ok(spoken.duration >= outro.at + 4 + 1, `the video ends ${spoken.duration - outro.at - 4} s after the line`);
  assert.ok(spoken.duration - spoken.outro >= plain.duration - plain.outro);
});

test("a line that was not measured stops the director", () => {
  assert.throws(
    () => place(record([{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }], [{ caption: null, say: "Hi." }])),
    /Hi\./,
  );
});

test("a spoken shot shows its line as subtitles, not its caption", () => {
  const timeline = schedule(record(
    [{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }],
    [{ caption: "Open a table.", ...said("Open a table, and it is just there.", 2) }],
  ));
  assert.equal(timeline.scenes[0].caption, null);
  assert.equal(timeline.subtitles.length, 1);
  const [sub] = timeline.subtitles;
  assert.equal(sub.text, "Open a table, and it is just there.");
  assert.equal(sub.from, timeline.voices[0].at);
  assert.ok(sub.to >= sub.from + 2, `${sub.to - sub.from} s`);
});

test("a scenario that says nothing keeps its captions", () => {
  const timeline = schedule(record([{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }]));
  assert.equal(timeline.scenes[0].caption, "Open a table.");
  assert.deepEqual(timeline.voices, []);
  assert.deepEqual(timeline.subtitles, []);
});

test("a long line is split into subtitles of six to nine words, timed by their words", () => {
  const line = "Every save is one transaction. It never overwrites someone else's change, and production always asks first.";
  const timeline = schedule(record(
    [{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }],
    [{ caption: null, ...said(line, 8) }],
  ));
  const subs = timeline.subtitles;
  const counts = subs.map(s => s.text.split(" ").length);
  assert.ok(subs.length >= 2, `${subs.length} subtitles`);
  assert.ok(counts.every(n => n >= 4 && n <= 10), counts.join());
  assert.equal(subs.map(s => s.text).join(" "), line);
  // A break falls after a full stop or a comma where one is near.
  assert.ok(subs[0].text.endsWith("transaction."), subs[0].text);
  const words = line.split(" ").length, at = timeline.voices[0].at;
  let from = at;
  subs.forEach((sub, i) => {
    assert.ok(Math.abs(sub.from - from) < 1e-9, `subtitle ${i} starts at ${sub.from}`);
    const share = (8 * counts[i]) / words;
    if (i < subs.length - 1) assert.ok(Math.abs(sub.to - sub.from - share) < 1e-9, `subtitle ${i}`);
    from = sub.to;
  });
  assert.ok(subs[subs.length - 1].to >= at + 8 - 1e-9);
});
