// The director: what the runner saw (events.json) made into a timeline.
// It is the one place a time is decided. The picture (index.html) and the
// sound (audio.js) only read what it returns. Pure: the same events give
// the same timeline. The design:
// docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md.
//
// render.cjs requires it; index.html loads it as a script (Director).

(function () {
  "use strict";

  const FPS = 60;
  // The window, in points: every rect is in them.
  const WIDTH = 1200, HEIGHT = 750;
  // Characters typed a second, on average.
  const TYPE_RATE = 13;
  // The camera frames what happens from BEHIND seconds ago to
  // LEAD + AHEAD seconds on, so it is there as the action starts.
  const LEAD = 0.25, AHEAD = 0.6, BEHIND = 0.4;
  const ZOOM = [1, 1.7];
  // The spring's angular frequency: it settles in about half a second.
  const OMEGA = 2 * Math.PI * 1.1;
  const READ_WORD = 0.3, READ_LEAST = 1.6;
  // How long a result stays after a scene's last step.
  const SETTLE = 0.6;
  const KEYCAP = 0.8;
  // The pause between the last typed character and a key (Enter, Run).
  const AFTER_TYPING = 0.3;
  // The spotlight lights an event a moment before it happens.
  const GLOW = 0.1;
  // Once the action has been quiet this long, and stays quiet for at least
  // QUIET, the camera pulls back and the spotlight lets go: the result is
  // seen whole, as it would be on the screen.
  const REVEAL = 0.9, QUIET = 2.4;
  // While it shows the whole window so, the camera drifts in this much a
  // second, so a held shot is not a frozen one.
  const DRIFT = 0.015;
  // Typing goes from a field's left edge: of a wide field (the SQL editor)
  // the camera frames that much of its left.
  const TYPE_SPAN = 400;

  const clamp = (x, a, b) => Math.min(b, Math.max(a, x));

  // Numbers in [0, 1) that `text` always starts the same (FNV-1a, then
  // mulberry32).
  function seeded(text) {
    let h = 2166136261;
    for (const c of text) h = Math.imul(h ^ c.codePointAt(0), 16777619) >>> 0;
    return () => {
      h = (h + 0x6d2b79f5) >>> 0;
      let t = h;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  // The first bar line at or after `t`.
  const onBar = (t, bar) => Math.ceil(t / bar - 1e-9) * bar;

  const centre = ([x0, y0, x1, y1]) => ({ x: (x0 + x1) / 2, y: (y0 + y1) / 2 });

  // The last of `list` (in time order) that `ok` holds for.
  function latest(list, ok) {
    let found = null;
    for (const item of list) if (ok(item)) found = item;
    return found;
  }

  // When each event happens, scene by scene, and when the scenes are.
  function place(record) {
    const beat = 60 / record.tempo, bar = 4 * beat;
    const random = seeded(record.title);
    const timed = [], scenes = [];
    let pointer = { x: WIDTH * 0.7, y: HEIGHT * 0.75 };
    // The intro is a bar.
    let t = bar;
    record.scenes.forEach((scene, index) => {
      const from = t;
      let now = from + beat / 2;
      for (const event of record.events.filter(e => e.scene === index)) {
        switch (event.kind) {
          case "point": {
            const to = centre(event.rect);
            const far = Math.hypot(to.x - pointer.x, to.y - pointer.y);
            const travel = clamp(0.35 + far / 1800, 0.35, 0.7);
            timed.push({ ...event, at: now + travel, leaves: now, origin: pointer, target: to });
            pointer = to;
            now += travel;
            break;
          }
          case "click":
            now += 0.12;
            timed.push({ ...event, at: now, where: pointer });
            now += 0.35;
            break;
          case "command":
          case "press":
            // A hand that has typed takes a moment before the key that ends it.
            if (timed.length && timed[timed.length - 1].kind === "type") now += AFTER_TYPING;
            timed.push({ ...event, at: now });
            now += 0.35;
            break;
          case "type": {
            const pause = /\s/.test(event.text) ? 1.4 : 1;
            now += (pause * (0.6 + 0.9 * random())) / TYPE_RATE;
            timed.push({ ...event, at: now });
            break;
          }
          case "hold":
            now += event.seconds;
            break;
          default:
            throw new Error(`an event the director does not know: ${event.kind}`);
        }
      }
      now += SETTLE;
      const words = scene.caption ? scene.caption.split(/\s+/).length : 0;
      const reading = scene.caption ? Math.max(READ_LEAST, READ_WORD * words) : 0;
      const to = onBar(Math.max(now, from + reading), bar);
      scenes.push({ from, to, caption: scene.caption || null });
      t = to;
    });
    return { timed, scenes, outro: t, beat, bar };
  }

  // The lit area round a box: a little room on each side.
  const lit = (x0, y0, x1, y1) => ({ x0: x0 - 12, y0: y0 - 8, x1: x1 + 12, y1: y1 + 8, alpha: 1 });

  // What the camera looks at to frame `rects`: their middle, zoomed so they
  // fill about half the picture, kept inside the window.
  function view(rects) {
    const x0 = Math.min(...rects.map(r => r[0])), y0 = Math.min(...rects.map(r => r[1]));
    const x1 = Math.max(...rects.map(r => r[2])), y1 = Math.max(...rects.map(r => r[3]));
    const zoom = clamp(0.5 / Math.max((x1 - x0) / WIDTH, (y1 - y0) / HEIGHT, 1e-3), ZOOM[0], ZOOM[1]);
    const half = { x: WIDTH / 2 / zoom, y: HEIGHT / 2 / zoom };
    return {
      x: clamp((x0 + x1) / 2, half.x, WIDTH - half.x),
      y: clamp((y0 + y1) / 2, half.y, HEIGHT - half.y),
      zoom,
      area: lit(x0, y0, x1, y1),
    };
  }

  const WHOLE = {
    x: WIDTH / 2, y: HEIGHT / 2, zoom: 1,
    area: { x0: 0, y0: 0, x1: WIDTH, y1: HEIGHT, alpha: 0 },
  };

  // Whether `want` is near enough where the camera is going that it stays:
  // inside the middle third of the view, at about the same zoom.
  function resting(target, want) {
    const third = { x: WIDTH / target.zoom / 6, y: HEIGHT / target.zoom / 6 };
    return Math.abs(want.x - target.x) < third.x
      && Math.abs(want.y - target.y) < third.y
      && Math.abs(want.zoom - target.zoom) / target.zoom < 0.15;
  }

  // One frame of a critically damped spring, solved exactly: from rest it
  // never overshoots.
  function spring(pos, vel, target, dt) {
    const e = Math.exp(-OMEGA * dt);
    for (const k of Object.keys(target)) {
      const x0 = pos[k] - target[k], v0 = vel[k], c = v0 + OMEGA * x0;
      pos[k] = target[k] + (x0 + c * dt) * e;
      vel[k] = (v0 - OMEGA * c * dt) * e;
    }
  }

  // What the camera frames of an event: its rect, but of a wide field
  // being typed in, only the left part, where the text goes.
  function aim(event) {
    const [x0, y0, x1, y1] = event.rect;
    return event.kind === "type" ? [x0, y0, Math.min(x1, x0 + TYPE_SPAN), y1] : event.rect;
  }

  // Since when the action has stopped at `t`, or null: the last thing
  // done is REVEAL seconds old, and nothing is done for another QUIET
  // seconds after it.
  function quiet(acts, t) {
    const last = latest(acts, e => e.at <= t);
    if (!last || t - last.at < REVEAL) return null;
    const next = acts.find(e => e.at > t);
    return !next || next.at - last.at >= QUIET ? last.at + REVEAL : null;
  }

  // Where the camera and the spotlight are, a frame at a time, and when
  // the camera set off for somewhere new. Until `hold`
  // the camera shows the whole window. The spotlight does not wait for the
  // camera: it lights the event of the moment. When the action stops, both
  // let go and show the whole window.
  function film(timed, outro, duration, hold = 0) {
    const framed = timed.filter(e => e.rect);
    const acts = timed.filter(e => e.kind !== "point");
    let target = WHOLE;
    const pos = { x: WHOLE.x, y: WHOLE.y, zoom: 1 }, vel = { x: 0, y: 0, zoom: 0 };
    const spot = { ...WHOLE.area }, spotVel = { x0: 0, y0: 0, x1: 0, y1: 0, alpha: 0 };
    const camera = [], spots = [], moves = [];
    for (let i = 0, n = Math.round(duration * FPS); i < n; i++) {
      const t = i / FPS;
      const ending = t >= outro - LEAD;
      const since = ending ? null : quiet(acts, t);
      const still = ending || since !== null;
      let want = null;
      if (still || t < hold) want = WHOLE;
      else {
        const near = framed.filter(e => e.at > t - BEHIND && e.at <= t + LEAD + AHEAD);
        if (near.length) want = view(near.map(aim));
      }
      if (want && want !== target && !(want !== WHOLE && target !== WHOLE && resting(target, want))) {
        moves.push({ at: t });
        target = want;
      }
      const drift = target === WHOLE && since !== null ? DRIFT * (t - since) : 0;
      spring(pos, vel, { x: target.x, y: target.y, zoom: target.zoom + drift }, 1 / FPS);
      const now = still ? null : latest(framed, e => e.at <= t + GLOW);
      const [x0, y0, x1, y1] = now ? now.rect : [];
      spring(spot, spotVel, now ? lit(x0, y0, x1, y1) : WHOLE.area, 1 / FPS);
      camera.push({ ...pos });
      spots.push({ ...spot });
    }
    return { camera, spot: spots, moves };
  }

  // Where the pointer is, frame by frame: it travels to each point on a
  // slight arc, eased in and out, presses for a click, and hides while
  // the keyboard types.
  function point(timed, duration) {
    const trips = timed.filter(e => e.kind === "point");
    const typing = timed.filter(e => e.kind === "type");
    const clicks = timed.filter(e => e.kind === "click");
    const frames = [];
    for (let i = 0, n = Math.round(duration * FPS); i < n; i++) {
      const t = i / FPS;
      const trip = latest(trips, e => e.leaves <= t);
      if (!trip) {
        frames.push({ x: 0, y: 0, shown: 0, press: 0 });
        continue;
      }
      const k = clamp((t - trip.leaves) / (trip.at - trip.leaves), 0, 1);
      const e = k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2;
      const a = trip.origin, b = trip.target;
      const dx = b.x - a.x, dy = b.y - a.y, length = Math.hypot(dx, dy) || 1;
      const bend = 0.12 * length;
      const control = { x: (a.x + b.x) / 2 + (dy / length) * bend, y: (a.y + b.y) / 2 - (dx / length) * bend };
      const u = 1 - e;
      const x = u * u * a.x + 2 * u * e * control.x + e * e * b.x;
      const y = u * u * a.y + 2 * u * e * control.y + e * e * b.y;
      const index = trips.indexOf(trip);
      const before = index === 0 ? -Infinity : trips[index - 1].at;
      const returns = index === 0 || typing.some(e => e.at > before && e.at <= trip.leaves);
      const appear = returns ? clamp((t - trip.leaves) / 0.15, 0, 1) : 1;
      // Hidden from the first key after the trip, as macOS hides it, until
      // the next trip.
      const typed = typing.find(e => e.at > trip.leaves && e.at <= t);
      const hidden = typed ? clamp((t - typed.at) / 0.15, 0, 1) : 0;
      const click = latest(clicks, e => e.at - 0.06 <= t);
      const press = click && t < click.at + 0.1 ? 1 : 0;
      frames.push({ x, y, shown: appear * (1 - hidden), press });
    }
    return frames;
  }

  // What is heard when: a key for each character and key, a click, a tick
  // under a keycap, a bell on the outro. The camera's moves are silent.
  function sounds(timed, outro, random) {
    const all = [];
    for (const e of timed) {
      if (e.kind === "type") all.push({ at: e.at, kind: "key", gain: 0.55 + 0.35 * random() });
      if (e.kind === "press") all.push({ at: e.at, kind: "key", gain: 0.9 });
      if (e.kind === "command") all.push({ at: e.at, kind: "key", gain: 1 }, { at: e.at, kind: "tick", gain: 0.8 });
      if (e.kind === "click") all.push({ at: e.at, kind: "click", gain: 1 });
    }
    all.push({ at: outro, kind: "bell", gain: 1 });
    return all.sort((a, b) => a.at - b.at);
  }

  // The timeline of a record: everything index.html and audio.js read.
  function schedule(record) {
    if (record.size[0] !== WIDTH || record.size[1] !== HEIGHT) {
      throw new Error(`the window is ${record.size.join("x")} points, not ${WIDTH}x${HEIGHT}`);
    }
    const { timed, scenes, outro, beat, bar } = place(record);
    const duration = outro + 2 * bar;
    // The camera holds the wide shot while the window rises, and a moment after.
    const intro = { from: 0, to: bar };
    const { camera, spot } = film(timed, outro, duration, intro.to + beat / 2);
    return {
      title: record.title,
      tempo: record.tempo,
      fps: FPS,
      beat,
      bar,
      duration,
      size: [WIDTH, HEIGHT],
      intro,
      outro,
      frames: [[0, 0], ...timed.map(e => [e.at, e.frame])].sort((a, b) => a[0] - b[0]),
      scenes,
      camera,
      spot,
      pointer: point(timed, duration),
      clicks: timed.filter(e => e.kind === "click").map(e => ({ at: e.at, x: e.where.x, y: e.where.y })),
      keycaps: timed.filter(e => e.kind === "command").map(e => ({ at: e.at, until: e.at + KEYCAP, keys: e.keys })),
      sounds: sounds(timed, outro, seeded(`${record.title} keys`)),
    };
  }

  const api = { schedule, place, film, seeded, FPS };
  if (typeof module !== "undefined") module.exports = api;
  else globalThis.Director = api;
})();
