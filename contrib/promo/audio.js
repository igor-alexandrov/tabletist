// The sound of a promotional video: music in the timeline's tempo and the
// effects on its events, rendered offline (no clock, no speakers) and
// returned as a 16-bit WAV in base64. index.html loads it after
// director.js; render.cjs calls renderAudio(timeline). Seeded: a timeline
// always sounds the same.

(function () {
  "use strict";

  const RATE = 48000;
  // The music's level, which the effects duck.
  const MUSIC = 0.55;
  // D major: Dmaj7, Bm7, Gmaj7, A6, a bar each, as MIDI notes; the outro
  // rings on Dmaj9.
  const CHORDS = [[50, 54, 57, 61], [47, 50, 54, 57], [43, 47, 50, 54], [45, 49, 52, 54]];
  const LAST = [50, 57, 61, 64];
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

    const music = ctx.createGain();
    music.gain.value = MUSIC;
    music.connect(mix());
    music.connect(room);
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
    return wav(await ctx.startRendering());
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

  // The pad alone under the intro; bass, kick, hats and the arpeggio from
  // the first scene; the last chord ringing out over the outro.
  function band(tools, into, timeline) {
    const { ctx, random } = tools;
    const { beat, bar, duration, outro } = timeline;
    const bed = ctx.createGain();
    bed.connect(into);
    const voice = mixer(ctx, bed);
    bed.gain.setValueAtTime(0, 0);
    bed.gain.linearRampToValueAtTime(1, timeline.intro.to * 0.8);
    bed.gain.setValueAtTime(1, duration - bar);
    bed.gain.linearRampToValueAtTime(0, duration - 0.05);
    // A scene ends on a half bar, so the outro may start in the middle of
    // one: the bar it starts in is cut there, and the last chord takes over.
    for (let at = 0; at < outro - 1e-6; at += bar) {
      const length = Math.min(bar, outro - at);
      const chord = CHORDS[Math.round(at / bar) % CHORDS.length];
      pad(ctx, voice, chord, at, length);
      if (at < timeline.intro.to - 1e-6) continue;
      bass(ctx, voice, chord[0] - 12, at, length);
      arp(ctx, voice, chord, at, beat, length);
      for (let k = 0; k * beat < length - 1e-6; k++) {
        if (k % 2 === 0) kick(ctx, voice, at + k * beat);
        if ((k + 0.5) * beat < length - 1e-6) hat(tools, voice, at + k * beat + beat / 2, random());
      }
    }
    pad(ctx, voice, LAST, outro, duration - outro);
  }

  // Detuned saws through a slow lowpass: warm, wide, quiet.
  function pad(ctx, into, chord, at, length) {
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.Q.value = 0.7;
    filter.frequency.setValueAtTime(700, at);
    filter.frequency.linearRampToValueAtTime(1500, at + length * 0.6);
    filter.frequency.linearRampToValueAtTime(900, at + length);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(0.05, at + 0.35);
    gain.gain.setValueAtTime(0.05, at + length - 0.1);
    gain.gain.linearRampToValueAtTime(0, at + length + 0.6);
    filter.connect(gain).connect(into());
    const strings = mixer(ctx, filter);
    for (const note of chord) {
      for (const cents of [-7, 7]) {
        const osc = ctx.createOscillator();
        osc.type = "sawtooth";
        osc.frequency.value = hz(note);
        osc.detune.value = cents;
        osc.connect(strings());
        osc.start(at);
        osc.stop(at + length + 0.7);
      }
    }
  }

  function bass(ctx, into, note, at, length) {
    const osc = ctx.createOscillator();
    osc.type = "sine";
    osc.frequency.value = hz(note);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(0.18, at + 0.02);
    gain.gain.setValueAtTime(0.18, at + length - 0.08);
    gain.gain.linearRampToValueAtTime(0, at + length);
    osc.connect(gain).connect(into());
    osc.start(at);
    osc.stop(at + length + 0.01);
  }

  // A felt kick: a sine falling in pitch, gone in a third of a second.
  function kick(ctx, into, at) {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.frequency.setValueAtTime(110, at);
    osc.frequency.exponentialRampToValueAtTime(42, at + 0.12);
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.5, at + 0.004);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.38);
    osc.connect(gain).connect(into());
    osc.start(at);
    osc.stop(at + 0.4);
  }

  function hat({ ctx, noise }, into, at, random) {
    const source = ctx.createBufferSource();
    source.buffer = noise;
    const filter = ctx.createBiquadFilter();
    filter.type = "highpass";
    filter.frequency.value = 7000;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.035 + 0.02 * random, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.05);
    source.connect(filter).connect(gain).connect(into());
    source.start(at, random * 0.5, 0.06);
  }

  // Eighths over the chord, an octave up, left and right in turn, for
  // `length` seconds.
  function arp(ctx, into, chord, at, beat, length) {
    [0, 1, 2, 3, 1, 2, 3, 2].forEach((n, i) => {
      if ((i * beat) / 2 < length - 1e-6) {
        pluck(ctx, into, hz(chord[n] + 12), at + (i * beat) / 2, i % 2 ? 0.35 : -0.35);
      }
    });
  }

  function pluck(ctx, into, freq, at, pan) {
    const osc = ctx.createOscillator();
    osc.type = "triangle";
    osc.frequency.value = freq;
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.setValueAtTime(4000, at);
    filter.frequency.exponentialRampToValueAtTime(600, at + 0.25);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.07, at + 0.005);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.4);
    const panner = ctx.createStereoPanner();
    panner.pan.value = pan;
    osc.connect(filter).connect(gain).connect(panner).connect(into());
    osc.start(at);
    osc.stop(at + 0.45);
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
