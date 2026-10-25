// The sound of a promotional video: the voice's lines, music in the
// timeline's tempo under them, and the effects on its events, rendered
// offline (no clock, no speakers) and returned as a 16-bit WAV in base64.
// index.html loads it after director.js; render.cjs calls
// renderAudio(timeline). Seeded: a timeline always sounds the same.

(function () {
  "use strict";

  const RATE = 48000;
  // The music's level, which the effects duck.
  const MUSIC = 0.85;
  // The voice's level, and how far the music goes down under it: 10 dB.
  const VOICE = 0.9;
  const UNDER = Math.pow(10, -10 / 20);
  // D major, bright and plain: D, A, Bm7, G(add9), a bar each, as MIDI
  // notes above middle C, with the bass's root under each; the outro rings
  // on D(add9).
  const CHORDS = [[62, 66, 69, 74], [61, 64, 69, 73], [62, 66, 69, 71], [62, 67, 69, 71]];
  const ROOTS = [38, 45, 47, 43];
  const LAST = [62, 66, 69, 76];
  const hz = note => 440 * Math.pow(2, (note - 69) / 12);

  async function renderAudio(timeline) {
    const ctx = new OfflineAudioContext(2, Math.ceil(timeline.duration * RATE), RATE);
    const random = Director.seeded(`${timeline.title} sound`);
    const noise = ctx.createBuffer(1, RATE, RATE);
    const white = noise.getChannelData(0);
    for (let i = 0; i < white.length; i++) white[i] = random() * 2 - 1;
    const tools = { ctx, noise, random };

    const bus = ctx.createDynamicsCompressor();
    bus.threshold.value = -12;
    bus.ratio.value = 3;
    bus.attack.value = 0.005;
    bus.release.value = 0.2;
    bus.connect(ctx.destination);
    const mix = mixer(ctx, bus);
    const room = ctx.createConvolver();
    room.buffer = impulse(ctx, 2.6, random);
    const wet = ctx.createGain();
    wet.gain.value = 0.3;
    room.connect(wet).connect(mix());

    // The music: its level ducked a little under each effect, then the
    // whole of it ducked under the voice.
    const music = ctx.createGain();
    music.gain.value = MUSIC;
    const under = ctx.createGain();
    music.connect(under);
    under.connect(mix());
    under.connect(room);
    const fx = ctx.createGain();
    fx.gain.value = 0.9;
    fx.connect(mix());
    const fxRoom = ctx.createGain();
    fxRoom.gain.value = 0.15;
    fx.connect(fxRoom).connect(room);

    band(tools, music, timeline);
    const effect = mixer(ctx, fx);
    for (const sound of timeline.sounds) {
      EFFECTS[sound.kind](tools, effect, sound.at, sound.gain);
      duck(music.gain, sound.at, sound.kind);
    }
    await speak(ctx, mix(), timeline.voices);
    beneath(under.gain, timeline.voices);
    return wav(await ctx.startRendering());
  }

  // The voice's lines, each its own WAV (render.cjs's, from macOS's say),
  // played at its time, dry: a voice in the room sounds far away. A
  // highpass takes out what is under a voice, and nothing of it is lost.
  async function speak(ctx, into, voices) {
    if (voices.length === 0) return;
    const level = ctx.createGain();
    level.gain.value = VOICE;
    const clean = ctx.createBiquadFilter();
    clean.type = "highpass";
    clean.frequency.value = 90;
    clean.connect(level).connect(into);
    const line = mixer(ctx, clean);
    for (const voice of voices) {
      const response = await fetch(voice.file);
      if (!response.ok) throw new Error(`${voice.file}: ${response.status}`);
      const source = ctx.createBufferSource();
      source.buffer = await ctx.decodeAudioData(await response.arrayBuffer());
      source.connect(line());
      source.start(voice.at);
    }
  }

  // The music goes down UNDER while the voice speaks: from a quarter of a
  // second before a line, back up over half a second after it. Lines less
  // than a second apart keep it down between them.
  function beneath(gain, voices) {
    const spans = [];
    for (const voice of voices) {
      const from = voice.at - 0.05, to = voice.at + voice.seconds + 0.1;
      const last = spans[spans.length - 1];
      if (last && from - last.to < 1) last.to = Math.max(last.to, to);
      else spans.push({ from, to });
    }
    gain.setValueAtTime(1, 0);
    for (const { from, to } of spans) {
      gain.setValueAtTime(1, Math.max(0, from - 0.25));
      gain.linearRampToValueAtTime(UNDER, from);
      gain.setValueAtTime(UNDER, to);
      gain.linearRampToValueAtTime(1, to + 0.5);
    }
  }

  // The same sum on every render. Chrome adds the connections into one input
  // in an order that changes from run to run, and floating point addition of
  // three or more numbers depends on that order, so many voices into one
  // node differ in their last bit between two renders. Here no input takes
  // more than two: one voice, and the sum of the voices added after it. The
  // function this returns gives each voice its own input: the `into` of the
  // voices and effects below.
  function mixer(ctx, into) {
    let last = into;
    return () => {
      const input = ctx.createGain();
      input.connect(last);
      last = input;
      return input;
    };
  }

  // The music ducks a little under each effect, and more under the big ones.
  function duck(gain, at, kind) {
    const depth = kind === "key" ? 0.92 : 0.7;
    gain.setTargetAtTime(MUSIC * depth, at, 0.01);
    gain.setTargetAtTime(MUSIC, at + 0.06, 0.15);
  }

  // A room: noise that dies away over `seconds`, each side its own.
  function impulse(ctx, seconds, random) {
    const length = Math.round(seconds * RATE);
    const buffer = ctx.createBuffer(2, length, RATE);
    for (let c = 0; c < 2; c++) {
      const data = buffer.getChannelData(c);
      for (let i = 0; i < length; i++) data[i] = (random() * 2 - 1) * Math.pow(1 - i / length, 3);
    }
    return buffer;
  }

  // The pad alone under the intro; bass, kick, clap, hats and the plucks
  // from the first scene; the last chord ringing out over the outro.
  function band(tools, into, timeline) {
    const { ctx, random } = tools;
    const { beat, bar, duration, outro } = timeline;
    const bed = ctx.createGain();
    bed.connect(into);
    const voice = mixer(ctx, bed);
    bed.gain.setValueAtTime(0, 0);
    bed.gain.linearRampToValueAtTime(1, Math.min(bar, timeline.intro.to) * 0.8);
    bed.gain.setValueAtTime(1, duration - bar);
    bed.gain.linearRampToValueAtTime(0, duration - 0.05);
    // Under a slide the drums and the plucks rest: the pad and the bass
    // hold it, a breath between two scenes.
    const slides = timeline.scenes.filter(scene => scene.slide);
    const rests = at => slides.some(card => at >= card.from - 1e-6 && at < card.to - 1e-6);
    // A scene ends on a half bar, so the outro may start in the middle of
    // one: the bar it starts in is cut there, and the last chord takes over.
    for (let at = 0; at < outro - 1e-6; at += bar) {
      const length = Math.min(bar, outro - at);
      const index = Math.round(at / bar) % CHORDS.length;
      const chord = CHORDS[index];
      pad(ctx, voice, chord, at, length);
      if (at < timeline.intro.to - 1e-6) continue;
      for (let k = 0; k * beat < length - 1e-6; k++) {
        const on = at + k * beat, off = on + beat / 2;
        bass(ctx, voice, ROOTS[index], on, beat);
        if (k % 2 === 0 && !rests(on)) kick(ctx, voice, on);
        if (k % 2 === 1 && !rests(on)) clap(tools, voice, on);
        if ((k + 0.5) * beat < length - 1e-6 && !rests(off)) hat(tools, voice, off, random());
      }
      plucks(ctx, voice, chord, at, beat, length, rests);
    }
    pad(ctx, voice, LAST, outro, duration - outro);
    bass(ctx, voice, ROOTS[0], outro, Math.min(bar, duration - outro));
  }

  // Detuned saws, open and airy: a lowpass high up that breathes over the
  // bar, and a highpass that leaves the low end to the bass and the voice.
  function pad(ctx, into, chord, at, length) {
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.Q.value = 0.5;
    filter.frequency.setValueAtTime(2200, at);
    filter.frequency.linearRampToValueAtTime(4800, at + length * 0.5);
    filter.frequency.linearRampToValueAtTime(2800, at + length);
    const air = ctx.createBiquadFilter();
    air.type = "highpass";
    air.frequency.value = 280;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(0.03, at + 0.2);
    gain.gain.setValueAtTime(0.03, at + length - 0.1);
    gain.gain.linearRampToValueAtTime(0, at + length + 0.5);
    filter.connect(air).connect(gain).connect(into());
    const strings = mixer(ctx, filter);
    for (const note of chord) {
      for (const cents of [-6, 6]) {
        const osc = ctx.createOscillator();
        osc.type = "sawtooth";
        osc.frequency.value = hz(note);
        osc.detune.value = cents;
        osc.connect(strings());
        osc.start(at);
        osc.stop(at + length + 0.6);
      }
    }
  }

  // A round bass on each beat, short enough to bounce.
  function bass(ctx, into, note, at, beat) {
    const osc = ctx.createOscillator();
    osc.type = "triangle";
    osc.frequency.value = hz(note);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.2, at + 0.01);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + beat * 0.8);
    osc.connect(gain).connect(into());
    osc.start(at);
    osc.stop(at + beat * 0.8 + 0.01);
  }

  // A light kick: a sine falling in pitch, gone in a quarter of a second.
  function kick(ctx, into, at) {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.frequency.setValueAtTime(130, at);
    osc.frequency.exponentialRampToValueAtTime(50, at + 0.1);
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.4, at + 0.004);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.26);
    osc.connect(gain).connect(into());
    osc.start(at);
    osc.stop(at + 0.28);
  }

  // A clap on beats 2 and 4: three quick bursts of noise and a short tail,
  // as hands that do not quite meet at once.
  function clap({ ctx, noise, random }, into, at) {
    const source = ctx.createBufferSource();
    source.buffer = noise;
    const filter = ctx.createBiquadFilter();
    filter.type = "bandpass";
    filter.frequency.value = 1600;
    filter.Q.value = 0.9;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.0001, at);
    for (const k of [0, 1, 2]) {
      const hit = at + k * 0.011;
      gain.gain.setValueAtTime(0.32, hit);
      gain.gain.exponentialRampToValueAtTime(0.04, hit + 0.009);
    }
    gain.gain.setValueAtTime(0.16, at + 0.033);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.2);
    source.connect(filter).connect(gain).connect(into());
    source.start(at, random() * 0.7, 0.22);
  }

  function hat({ ctx, noise }, into, at, random) {
    const source = ctx.createBufferSource();
    source.buffer = noise;
    const filter = ctx.createBiquadFilter();
    filter.type = "highpass";
    filter.frequency.value = 8000;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.04 + 0.02 * random, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.045);
    source.connect(filter).connect(gain).connect(into());
    source.start(at, random * 0.5, 0.05);
  }

  // Eighths over the chord, up and down it, left and right in turn, for
  // `length` seconds, but none where the music `rests`.
  function plucks(ctx, into, chord, at, beat, length, rests) {
    [0, 2, 1, 3, 2, 1, 3, 2].forEach((n, i) => {
      const on = at + (i * beat) / 2;
      if ((i * beat) / 2 < length - 1e-6 && !rests(on)) {
        pluck(ctx, into, hz(chord[n] + 12), on, i % 2 ? 0.3 : -0.3);
      }
    });
  }

  // A crisp pluck: a square and a saw an octave apart, its brightness
  // snapping shut in a tenth of a second.
  function pluck(ctx, into, freq, at, pan) {
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.Q.value = 2;
    filter.frequency.setValueAtTime(7000, at);
    filter.frequency.exponentialRampToValueAtTime(900, at + 0.12);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.045, at + 0.003);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.28);
    const panner = ctx.createStereoPanner();
    panner.pan.value = pan;
    filter.connect(gain).connect(panner).connect(into());
    const tone = mixer(ctx, filter);
    for (const [type, times] of [["square", 1], ["sawtooth", 2]]) {
      const osc = ctx.createOscillator();
      osc.type = type;
      osc.frequency.value = freq * times;
      osc.connect(tone());
      osc.start(at);
      osc.stop(at + 0.3);
    }
  }

  // A short burst of noise around `freq`: half of a mouse click.
  function tap({ ctx, noise, random }, into, at, freq, level) {
    const source = ctx.createBufferSource();
    source.buffer = noise;
    const filter = ctx.createBiquadFilter();
    filter.type = "bandpass";
    filter.frequency.value = freq;
    filter.Q.value = 2;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(level, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.012);
    source.connect(filter).connect(gain).connect(into());
    source.start(at, random() * 0.9, 0.02);
  }

  const EFFECTS = {
    // Each key a little different: a click of noise and a body under it.
    key({ ctx, noise, random }, into, at, gain) {
      const source = ctx.createBufferSource();
      source.buffer = noise;
      const filter = ctx.createBiquadFilter();
      filter.type = "bandpass";
      filter.frequency.value = 2500 + 1500 * random();
      filter.Q.value = 1.2;
      const level = ctx.createGain();
      level.gain.setValueAtTime(0.22 * gain, at);
      level.gain.exponentialRampToValueAtTime(0.0001, at + 0.03);
      const panner = ctx.createStereoPanner();
      panner.pan.value = (random() - 0.5) * 0.3;
      source.connect(filter).connect(level).connect(panner).connect(into());
      source.start(at, random() * 0.9, 0.04);
      const body = ctx.createOscillator();
      body.frequency.value = 180 + 60 * random();
      const thud = ctx.createGain();
      thud.gain.setValueAtTime(0.12 * gain, at);
      thud.gain.exponentialRampToValueAtTime(0.0001, at + 0.025);
      body.connect(thud).connect(into());
      body.start(at);
      body.stop(at + 0.03);
    },
    // Down, and up again.
    click(tools, into, at, gain) {
      tap(tools, into, at, 2200, 0.8 * gain);
      tap(tools, into, at + 0.07, 2900, 0.45 * gain);
    },
    // A low, soft note under a keycap.
    tick({ ctx }, into, at, gain) {
      for (const [times, level] of [[1, 0.14], [2, 0.05], [3, 0.025]]) {
        const osc = ctx.createOscillator();
        osc.frequency.value = 220 * times;
        const amp = ctx.createGain();
        amp.gain.setValueAtTime(0.0001, at);
        amp.gain.exponentialRampToValueAtTime(level * gain, at + 0.004);
        amp.gain.exponentialRampToValueAtTime(0.0001, at + 0.15);
        osc.connect(amp).connect(into());
        osc.start(at);
        osc.stop(at + 0.16);
      }
    },
    // An FM bell on D, and its octave below.
    bell({ ctx }, into, at, gain) {
      const f = hz(86);
      const carrier = ctx.createOscillator();
      carrier.frequency.value = f;
      const modulator = ctx.createOscillator();
      modulator.frequency.value = f * 3.5;
      const depth = ctx.createGain();
      depth.gain.setValueAtTime(f * 2, at);
      depth.gain.exponentialRampToValueAtTime(1, at + 1.5);
      modulator.connect(depth).connect(carrier.frequency);
      const level = ctx.createGain();
      level.gain.setValueAtTime(0.0001, at);
      level.gain.exponentialRampToValueAtTime(0.18 * gain, at + 0.005);
      level.gain.exponentialRampToValueAtTime(0.0001, at + 2.8);
      carrier.connect(level).connect(into());
      const low = ctx.createOscillator();
      low.frequency.value = hz(74);
      const lowLevel = ctx.createGain();
      lowLevel.gain.setValueAtTime(0.0001, at);
      lowLevel.gain.exponentialRampToValueAtTime(0.08 * gain, at + 0.01);
      lowLevel.gain.exponentialRampToValueAtTime(0.0001, at + 2.5);
      low.connect(lowLevel).connect(into());
      for (const osc of [carrier, modulator, low]) {
        osc.start(at);
        osc.stop(at + 3);
      }
    },
  };

  // The rendered sound as a 16-bit PCM WAV, in base64.
  function wav(buffer) {
    const channels = buffer.numberOfChannels, frames = buffer.length, size = frames * channels * 2;
    const view = new DataView(new ArrayBuffer(44 + size));
    const text = (at, s) => [...s].forEach((c, i) => view.setUint8(at + i, c.charCodeAt(0)));
    text(0, "RIFF");
    view.setUint32(4, 36 + size, true);
    text(8, "WAVE");
    text(12, "fmt ");
    view.setUint32(16, 16, true);
    view.setUint16(20, 1, true);
    view.setUint16(22, channels, true);
    view.setUint32(24, RATE, true);
    view.setUint32(28, RATE * channels * 2, true);
    view.setUint16(32, channels * 2, true);
    view.setUint16(34, 16, true);
    text(36, "data");
    view.setUint32(40, size, true);
    const data = Array.from({ length: channels }, (_, c) => buffer.getChannelData(c));
    for (let i = 0, at = 44; i < frames; i++) {
      for (let c = 0; c < channels; c++, at += 2) {
        view.setInt16(at, Math.max(-1, Math.min(1, data[c][i])) * 32767, true);
      }
    }
    const bytes = new Uint8Array(view.buffer);
    let binary = "";
    for (let i = 0; i < bytes.length; i += 0x8000) {
      binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    }
    return btoa(binary);
  }

  globalThis.renderAudio = renderAudio;
})();
