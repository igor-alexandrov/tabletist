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
  const TYPE_RATE = 18;
  // The camera frames what happens from BEHIND seconds ago to
  // LEAD + AHEAD seconds on, so it is there as the action starts.
  const LEAD = 0.25, AHEAD = 0.6, BEHIND = 0.4;
  // Gentle: the app stays whole enough to read, as a person would show it.
  const ZOOM = [1, 1.25];
  // The spring's angular frequency: it settles in about half a second.
  const OMEGA = 2 * Math.PI * 1.1;
  const READ_WORD = 0.3, READ_LEAST = 1.6;
  // A slide's time: its words read, faster than a caption read while the
  // app moves, and the card's coming and going, but never less than
  // SLIDE_LEAST.
  const SLIDE_WORD = 0.22, SLIDE_BUILD = 0.9, SLIDE_LEAST = 2.2;
  // A line the voice says starts this far into its shot, and its scene
  // keeps this much room after it.
  const VOICE_IN = 0.2, VOICE_ROOM = 0.4;
  // A slide's line starts once its headline is up: index.html raises the
  // headline from HEADLINE.start into the slide, a word every
  // HEADLINE.word seconds, each taking HEADLINE.rise to be read.
  const HEADLINE = { start: 0.3, word: 0.07, rise: 0.35 };
  // The intro's and the outro's lines start once the card's big line is in.
  const CARD_VOICE = 0.6;
  // How long the outro stays after its line.
  const OUTRO_TAIL = 1.2;
  // A subtitle holds this many words at most; a longer line is split.
  const SUBTITLE_WORDS = 9;
  const LEADING = /^(a|an|and|as|at|by|for|from|in|it|like|of|on|or|so|the|to|with|you|your)$/i;
  // How long a result stays after a scene's last step.
  const SETTLE = 0.4;
  // How long a click, a command or a key holds before the next step.
  const STEP = 0.25;
  const KEYCAP = 0.8;
  // The pause between the last typed character and a key (Enter, Run).
  const AFTER_TYPING = 0.3;
  // The spotlight lights an event a moment before it happens.
  const GLOW = 0.1;
  // Once the action has been quiet this long, and stays quiet for at least
  // QUIET, the camera pulls back and the spotlight lets go: the result is
  // seen whole, as it would be on the screen.
  const REVEAL = 0.6, QUIET = 1.6;
  // While it shows the whole window so, the camera drifts in this much a
  // second, so a held shot is not a frozen one.
  const DRIFT = 0.015;
  // Typing goes from a field's left edge: of a wide field (the SQL editor)
  // the camera frames that much of its left.
  const TYPE_SPAN = 400;
  // A scene with `beats` lasts exactly that many. A shot of that kind is a
  // cut: its steps are squeezed into all but TAIL of it, so the result
  // shows, and the camera, set straight on its framing at the cut, pushes
  // in by PUSH over it.
  const TAIL = 0.15, PUSH = 0.06;
  // An intro with `beats` is a hook: its line starts at once.
  const HOOK_VOICE = 0.2;
  // In a cut, typing is a flurry: a key is heard at most this often.
  const KEY_GAP = 0.05;
  // A cut frames its result, close: what was done in its last LATE share,
  // filling FILL of the picture above the band, and zoomed up to REEL_ZOOM
  // by the end of its push. A narrated video keeps to ZOOM.
  const LATE = 0.6, FILL = 0.9, REEL_ZOOM = 1.9;
  // The hook's title rises word by word (index.html), from
  // HOOK_WORDS.start, a word every HOOK_WORDS.word seconds, each taking
  // HOOK_WORDS.rise. A number in it counts up from 0 to itself, eased
  // out, from the hook's first frame,
  // landing COUNT.land before the voice says it: COUNT.said of the way
  // into the line (Ava says "fifty" from 0.46 s of the 2.0 of "More than
  // fifty new features."). With no line, it counts for COUNT.length.
  const HOOK_WORDS = { start: 0, word: 0.04, rise: 0.35 };
  const COUNT = { said: 0.23, land: 0.03, length: 0.8 };

  // The picture index.html draws, in pixels: the window at rest fills it
  // above a band kept for the subtitles, MARGIN from its top.
  const PICTURE = { width: 1920, height: 1080, band: 156, margin: 24 };
  // The picture's pixels to a point of the window at zoom 1, and where a
  // zoomed-in point is brought: the middle of the space above the band.
  const PX = (PICTURE.height - PICTURE.band - PICTURE.margin) / HEIGHT;
  const MIDDLE = PICTURE.margin + (HEIGHT / 2) * PX;
  // How much of the point's way to that middle a zoom takes it: all of it
  // from PULLED on, where the window, zoomed, covers the picture.
  const PULLED = 4 / 3;
  const pull = zoom => clamp((zoom - 1) / (PULLED - 1), 0, 1);
  // How far the picture reaches round that point, in points at zoom 1.
  const REACH = { across: PICTURE.width / 2 / PX, up: MIDDLE / PX, down: (PICTURE.height - MIDDLE) / PX };

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

  // The first line of a grid of `step` seconds at or after `t`.
  const onGrid = (t, step) => Math.ceil(t / step - 1e-9) * step;

  const centre = ([x0, y0, x1, y1]) => ({ x: (x0 + x1) / 2, y: (y0 + y1) / 2 });

  // The last of `list` (in time order) that `ok` holds for.
  function latest(list, ok) {
    let found = null;
    for (const item of list) if (ok(item)) found = item;
    return found;
  }

  const words = text => text.trim().split(/\s+/).length;

  // What the voice says of `owner` (a scene, the intro, the outro) and how
  // long it takes, or null where it says nothing. render.cjs measures each
  // line before the director runs.
  function spoken(owner) {
    if (!owner || !owner.say) return null;
    if (!owner.speech) throw new Error(`a line to say was not measured: "${owner.say}"`);
    return { text: owner.say, file: owner.speech.file, seconds: owner.speech.seconds };
  }

  // A line said from `at`: when it ends, with its room after.
  const heard = (voice, at) => (voice ? at + voice.seconds + VOICE_ROOM : 0);

  // Whether a scene, the intro or the outro lasts a fixed number of beats.
  const fixed = owner => Boolean(owner && owner.beats);

  // The count of the number in a hook's `title` (its first word of
  // digits), timed to land just before `voice`, at that time, says it; null
  // where the title has no number.
  function count(title, voice) {
    const number = title.split(/\s+/).find(word => /^\d+$/.test(word));
    if (!number) return null;
    const to = voice ? voice.at + COUNT.said * voice.seconds - COUNT.land : COUNT.length;
    return { from: Math.max(0, to - COUNT.length), to, value: Number(number) };
  }

  // When each event happens, scene by scene, and when the scenes are, and
  // the lines the voice says.
  function place(record) {
    const beat = 60 / record.tempo, bar = 4 * beat;
    const random = seeded(record.title);
    const timed = [], scenes = [], voices = [];
    let pointer = { x: WIDTH * 0.7, y: HEIGHT * 0.75 };
    // The intro is a bar, or as long as its line and the beat its card
    // takes to leave; with beats, exactly that many, its line running on
    // into the first scene if it is longer.
    const opening = spoken(record.intro);
    let t;
    if (fixed(record.intro)) {
      if (opening) voices.push({ at: HOOK_VOICE, ...opening });
      t = record.intro.beats * beat;
    } else {
      if (opening) voices.push({ at: CARD_VOICE, ...opening });
      t = Math.max(bar, onGrid(heard(opening, CARD_VOICE) + beat, bar / 2));
    }
    const named = record.intro || {};
    const intro = { from: 0, to: t, title: named.title || null, text: named.text || null };
    // The hook: its words' rise, and the count of its number.
    if (fixed(record.intro) && named.title) {
      intro.words = HOOK_WORDS;
      intro.count = count(named.title, opening ? voices[0] : null);
    }
    record.scenes.forEach((scene, index) => {
      const from = t;
      const voice = spoken(scene);
      // A flash card: exactly its beats, whatever it says.
      if (scene.slide && fixed(scene)) {
        if (voice) voices.push({ at: from + VOICE_IN, ...voice });
        const to = from + scene.beats * beat;
        scenes.push({ from, to, caption: null, slide: { title: scene.slide, text: scene.text || null }, beats: scene.beats });
        t = to;
        return;
      }
      // A slide: the app is away, and only the words take time.
      if (scene.slide) {
        const at = from + HEADLINE.start + HEADLINE.word * (words(scene.slide) - 1) + HEADLINE.rise;
        if (voice) voices.push({ at, ...voice });
        const reading = from + Math.max(SLIDE_LEAST, SLIDE_WORD * words(`${scene.slide} ${scene.text || ""}`) + SLIDE_BUILD);
        const to = onGrid(Math.max(reading, heard(voice, at)), bar / 2);
        scenes.push({ from, to, caption: null, slide: { title: scene.slide, text: scene.text || null }, beats: null });
        t = to;
        return;
      }
      if (voice) voices.push({ at: from + VOICE_IN, ...voice });
      // A cut's steps are placed from its start, then squeezed into it.
      const cut = fixed(scene);
      const own = [];
      let now = cut ? 0 : from + beat / 2;
      for (const event of record.events.filter(e => e.scene === index)) {
        switch (event.kind) {
          case "point": {
            const to = centre(event.rect);
            const far = Math.hypot(to.x - pointer.x, to.y - pointer.y);
            const travel = clamp(0.28 + far / 2200, 0.28, 0.55);
            own.push({ ...event, at: now + travel, leaves: now, origin: pointer, target: to });
            pointer = to;
            now += travel;
            break;
          }
          case "click":
            now += 0.1;
            own.push({ ...event, at: now, where: pointer });
            now += STEP;
            break;
          case "command":
          case "press":
            // A hand that has typed takes a moment before the key that ends it.
            const before = own.length ? own[own.length - 1] : timed[timed.length - 1];
            if (before && before.kind === "type") now += AFTER_TYPING;
            own.push({ ...event, at: now });
            now += STEP;
            break;
          case "type": {
            const pause = /\s/.test(event.text) ? 1.4 : 1;
            now += (pause * (0.6 + 0.9 * random())) / TYPE_RATE;
            own.push({ ...event, at: now });
            break;
          }
          case "hold":
            now += event.seconds;
            break;
          default:
            throw new Error(`an event the director does not know: ${event.kind}`);
        }
      }
      // A line said takes the caption's place: the subtitles show it.
      const caption = voice ? null : scene.caption || null;
      const label = scene.label || null;
      if (cut) {
        // Exactly its beats: what took longer than all but its TAIL is
        // made faster, the pointer's trips with it; what is quicker keeps
        // its pace.
        const to = from + scene.beats * beat;
        const k = now > 0 ? Math.min(1, ((1 - TAIL) * (to - from)) / now) : 1;
        const squeezed = x => from + x * k;
        for (const event of own) {
          const placed = { ...event, at: squeezed(event.at), cut: true };
          if (event.kind === "point") placed.leaves = squeezed(event.leaves);
          timed.push(placed);
        }
        scenes.push({ from, to, caption, label, slide: null, beats: scene.beats });
        t = to;
        return;
      }
      timed.push(...own);
      now += SETTLE;
      const reading = caption ? Math.max(READ_LEAST, READ_WORD * words(caption)) : 0;
      // A scene ends on a half bar: the next starts on the beat, without
      // waiting out a whole bar.
      const to = onGrid(Math.max(now, from + reading, heard(voice, from + VOICE_IN)), bar / 2);
      scenes.push({ from, to, caption, label, slide: null, beats: null });
      t = to;
    });
    const closing = spoken(record.outro);
    if (closing) voices.push({ at: t + CARD_VOICE, ...closing });
    // The outro is a bar and a half, or as long as its line and a breath;
    // with beats, exactly that many.
    const ending = closing ? onGrid(CARD_VOICE + closing.seconds + OUTRO_TAIL, bar / 2) : 0;
    const duration = t + (fixed(record.outro) ? record.outro.beats * beat : Math.max(1.5 * bar, ending));
    // The hard cuts: wherever something of fixed length ends and another
    // of fixed length, or the outro, starts. No curtain moves there, and
    // the camera does not spring across it.
    const parts = [{ to: intro.to, fixed: fixed(record.intro) }, ...scenes.map(scene => ({ to: scene.to, fixed: Boolean(scene.beats) }))];
    const cuts = parts.filter((part, i) => part.fixed && (i + 1 === parts.length || parts[i + 1].fixed)).map(part => part.to);
    return { timed, scenes, voices, intro, outro: t, duration, beat, bar, cuts };
  }

  // A line's words in pieces of SUBTITLE_WORDS at most and 4 at least,
  // about 7, a break after a sentence or a comma where one is near: the
  // split that costs least, each piece costing its distance from 7
  // squared and 2, less 8 for ending a sentence, 4 a clause, and 4 more
  // for ending on a little word that leads into the next ("and", "the").
  function chunks(text) {
    const all = text.trim().split(/\s+/), n = all.length;
    if (n <= SUBTITLE_WORDS) return [all];
    const cost = (from, to) => {
      const end = to < n ? all[to - 1] : "";
      const bonus = /[.!?]$/.test(end) ? 8 : /[,;:]$/.test(end) ? 4 : 0;
      const hanging = LEADING.test(end) ? 4 : 0;
      return (to - from - 7) ** 2 + 2 - bonus + hanging;
    };
    // best[i]: the cheapest split of the first i words, and its last break.
    const best = [{ cost: 0, from: null }];
    for (let to = 1; to <= n; to++) {
      best[to] = { cost: Infinity, from: null };
      for (let from = Math.max(0, to - SUBTITLE_WORDS); from <= to - 4; from++) {
        if (best[from].cost === Infinity) continue;
        const c = best[from].cost + cost(from, to);
        if (c < best[to].cost) best[to] = { cost: c, from };
      }
    }
    const pieces = [];
    for (let to = n; to > 0; to = best[to].from) pieces.unshift(all.slice(best[to].from, to));
    return pieces;
  }

  // The subtitles of the lines: each line in chunks, each chunk timed by
  // its share of the line's words. The last stays until the line ends.
  function subtitles(voices) {
    const all = [];
    for (const voice of voices) {
      const pieces = chunks(voice.text), total = pieces.flat().length;
      let from = voice.at;
      pieces.forEach((piece, k) => {
        const to = k === pieces.length - 1 ? voice.at + voice.seconds : from + (voice.seconds * piece.length) / total;
        all.push({ from, to, text: piece.join(" ") });
        from = to;
      });
    }
    return all;
  }

  // The lit area round a box: a little room on each side.
  const lit = (x0, y0, x1, y1) => ({ x0: x0 - 12, y0: y0 - 8, x1: x1 + 12, y1: y1 + 8, alpha: 1 });

  // What the camera looks at to frame `rects`: their middle, zoomed so they
  // fill about half the picture (`most` at most), kept inside the window.
  function view(rects, most = ZOOM[1]) {
    const x0 = Math.min(...rects.map(r => r[0])), y0 = Math.min(...rects.map(r => r[1]));
    const x1 = Math.max(...rects.map(r => r[2])), y1 = Math.max(...rects.map(r => r[3]));
    const zoom = clamp(0.5 / Math.max((x1 - x0) / WIDTH, (y1 - y0) / HEIGHT, 1e-3), ZOOM[0], most);
    const half = { x: WIDTH / 2 / zoom, y: HEIGHT / 2 / zoom };
    return {
      x: clamp((x0 + x1) / 2, half.x, WIDTH - half.x),
      y: clamp((y0 + y1) / 2, half.y, HEIGHT - half.y),
      zoom,
      area: lit(x0, y0, x1, y1),
    };
  }

  // The box round `rects`.
  function union(rects) {
    return [
      Math.min(...rects.map(r => r[0])), Math.min(...rects.map(r => r[1])),
      Math.max(...rects.map(r => r[2])), Math.max(...rects.map(r => r[3])),
    ];
  }

  // What the camera looks at to frame a cut's result: `rects` filling
  // FILL of the picture above the band, zoomed in at least until the
  // window covers the picture (PULLED) and at most so that the push ends
  // on REEL_ZOOM, and kept where the picture shows nothing but the window.
  function closeup(rects) {
    const [x0, y0, x1, y1] = union(rects);
    const fit = FILL * Math.min((2 * REACH.across) / Math.max(x1 - x0, 1), HEIGHT / Math.max(y1 - y0, 1));
    const zoom = clamp(fit, PULLED, REEL_ZOOM / (1 + PUSH));
    return {
      x: clamp((x0 + x1) / 2, REACH.across / zoom, WIDTH - REACH.across / zoom),
      y: clamp((y0 + y1) / 2, REACH.up / zoom, HEIGHT - REACH.down / zoom),
      zoom,
      area: lit(x0, y0, x1, y1),
    };
  }

  // What a cut frames of an event: its rect, but of a key in a wide field
  // (the SQL editor: typing, Format, Run), only the left part, where the
  // text is.
  function near(event) {
    const [x0, y0, x1, y1] = event.rect;
    return event.kind === "point" || event.kind === "click" ? event.rect : [x0, y0, Math.min(x1, x0 + TYPE_SPAN), y1];
  }

  // A cut's framing, `events` its own: its result. The rects of what it
  // did in its last LATE; else of all it did; else, where nothing it did
  // has one (a key with nothing focused), the box its last change on
  // screen took (the runner's `changed`); else the whole window.
  function result(events, scene) {
    const late = scene.to - LATE * (scene.to - scene.from);
    const rects = events.filter(e => e.rect);
    const ending = rects.filter(e => e.at >= late - 1e-9);
    const changes = events.filter(e => e.changed);
    if (ending.length) return closeup(ending.map(near));
    if (rects.length) return closeup(rects.map(near));
    if (changes.length) return closeup([changes[changes.length - 1].changed]);
    return WHOLE;
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
  // let go and show the whole window. In each of `shots`, the cuts, it is
  // set straight on the cut's framing at its first frame, and pushes in.
  function film(timed, outro, duration, hold = 0, away = [], shots = []) {
    const framed = timed.filter(e => e.rect);
    const acts = timed.filter(e => e.kind !== "point");
    let target = WHOLE;
    const pos = { x: WHOLE.x, y: WHOLE.y, zoom: 1 }, vel = { x: 0, y: 0, zoom: 0 };
    const spot = { ...WHOLE.area }, spotVel = { x0: 0, y0: 0, x1: 0, y1: 0, alpha: 0 };
    const camera = [], spots = [], moves = [];
    let cutting = null;
    for (let i = 0, n = Math.round(duration * FPS); i < n; i++) {
      const t = i / FPS;
      const shot = shots.find(s => t >= s.from - 1e-9 && t < s.to - 1e-9) || null;
      if (shot) {
        if (shot !== cutting) {
          moves.push({ at: t });
          target = shot.view;
          for (const k of Object.keys(vel)) vel[k] = 0;
        }
        const k = (t - shot.from) / (shot.to - shot.from);
        Object.assign(pos, { x: shot.view.x, y: shot.view.y, zoom: shot.view.zoom * (1 + PUSH * k) });
      }
      cutting = shot;
      // For the outro and for a slide, the window is away: the camera
      // shows it whole, so it comes back as it left.
      const ending = t >= outro - LEAD || away.some(card => t >= card.from - LEAD && t < card.to);
      const since = ending ? null : quiet(acts, t);
      const still = ending || since !== null;
      let want = null;
      if (still || t < hold) want = WHOLE;
      else {
        const near = framed.filter(e => e.at > t - BEHIND && e.at <= t + LEAD + AHEAD);
        if (near.length) want = view(near.map(aim));
      }
      if (!shot && want && want !== target && !(want !== WHOLE && target !== WHOLE && resting(target, want))) {
        moves.push({ at: t });
        target = want;
      }
      const drift = target === WHOLE && since !== null ? DRIFT * (t - since) : 0;
      if (!shot) spring(pos, vel, { x: target.x, y: target.y, zoom: target.zoom + drift }, 1 / FPS);
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
  // under a keycap (none in a cut), a soft bell as a slide comes in, a bell on the outro,
  // and in a reel a hit with it. In a cut, typing is heard a key every
  // KEY_GAP at most. The camera's moves are silent.
  function sounds(timed, outro, slides, random, reel) {
    const all = [];
    let key = -Infinity;
    for (const e of timed) {
      if (e.kind === "type" && !(e.cut && e.at - key < KEY_GAP)) {
        all.push({ at: e.at, kind: "key", gain: 0.55 + 0.35 * random() });
        key = e.at;
      }
      if (e.kind === "press") all.push({ at: e.at, kind: "key", gain: 0.9 });
      if (e.kind === "command") all.push({ at: e.at, kind: "key", gain: 1 });
      // The tick goes with a keycap, which a cut has none of.
      if (e.kind === "command" && !e.cut) all.push({ at: e.at, kind: "tick", gain: 0.8 });
      if (e.kind === "press" || e.kind === "command") key = e.at;
      if (e.kind === "click") all.push({ at: e.at, kind: "click", gain: 1 });
    }
    for (const card of slides) all.push({ at: card.from, kind: "bell", gain: 0.45 });
    all.push({ at: outro, kind: "bell", gain: 1 });
    if (reel) all.push({ at: outro, kind: "hit", gain: 1 });
    return all.sort((a, b) => a.at - b.at);
  }

  // The timeline of a record: everything index.html and audio.js read.
  function schedule(record) {
    if (record.size[0] !== WIDTH || record.size[1] !== HEIGHT) {
      throw new Error(`the window is ${record.size.join("x")} points, not ${WIDTH}x${HEIGHT}`);
    }
    const { timed, scenes, voices, intro, outro, duration, beat, bar, cuts } = place(record);
    // The camera holds the wide shot while the window rises, and a moment after.
    const slides = scenes.filter(scene => scene.slide);
    // The cuts, each framed on its result.
    const shots = scenes.flatMap((scene, index) => {
      if (!scene.beats || scene.slide) return [];
      return [{ from: scene.from, to: scene.to, view: result(timed.filter(e => e.scene === index), scene) }];
    });
    const { camera, spot } = film(timed, outro, duration, intro.to + beat / 2, slides, shots);
    const reel = scenes.some(scene => scene.beats);
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
      // A cut has no keycap: its label stands alone.
      keycaps: timed.filter(e => e.kind === "command" && !e.cut).map(e => ({ at: e.at, until: e.at + KEYCAP, keys: e.keys })),
      sounds: sounds(timed, outro, slides.filter(card => !card.beats), seeded(`${record.title} keys`), reel),
      reel,
      cuts,
      voices: voices.map(({ at, file, seconds, text }) => ({ at, file, seconds, text })),
      // A reel's lines are heard, not read: its cards say the same.
      subtitles: reel ? [] : subtitles(voices),
    };
  }

  const api = { schedule, place, film, seeded, FPS, PICTURE, pull, REACH, WIDTH, HEIGHT };
  if (typeof module !== "undefined") module.exports = api;
  else globalThis.Director = api;
})();
