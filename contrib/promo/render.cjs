// Renders a promotional video: the director's timeline drawn by index.html
// in a headless Chrome, a screenshot a frame, its sound by audio.js, encoded
// and mixed by ffmpeg.
// contrib/promo/make runs it:
//
//   node contrib/promo/render.cjs <work dir> <out.mp4> [--final] [--still <seconds>...]
//
// The work dir is what the runner (src/promo/runner.rs) wrote: events.json
// and frames/. --final renders four frames for each one and has ffmpeg
// blend them (motion blur). --still writes those moments as PNG beside the
// video's path instead.
//
// The lines the voice says are spoken first, by macOS's `say`, into
// <work dir>/voice/, and measured, so the director can time the scenes
// round them.

"use strict";

const { execFileSync, spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const puppeteer = require("puppeteer-core");
const { schedule } = require("./director.js");

const chrome =
  process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

// The names of the voices `say` has, as `say -v '?'` lists them:
// "Ava (Premium)       en_US    # Hello! My name is Ava."
function installed() {
  return execFileSync("say", ["-v", "?"], { encoding: "utf8" })
    .split("\n")
    .map(line => /^(.+?)\s+[a-z]{2,3}_\w+\s+#/.exec(line))
    .filter(Boolean)
    .map(found => found[1]);
}

// Each line the record says spoken into <work>/voice/NN.aiff, in the
// record's voice, then made a 48 kHz WAV for Chrome (which decodes no
// AIFF), and measured with ffprobe. Each line's owner (a scene, the intro,
// the outro) gets `speech`: the WAV's file:// URL and its seconds.
function speak(record, work) {
  const owners = [record.intro, ...record.scenes, record.outro].filter(owner => owner && owner.say);
  if (owners.length === 0) return;
  if (process.platform !== "darwin") {
    throw new Error("the voice is macOS's say: a scenario with lines to say renders on a Mac");
  }
  if (!installed().includes(record.voice)) {
    throw new Error(
      `the voice "${record.voice}" is not installed. Add it in System Settings > Accessibility > ` +
        "Spoken Content > System voice > Manage Voices, or name an installed one (say -v '?').",
    );
  }
  const dir = path.join(work, "voice");
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir);
  owners.forEach((owner, index) => {
    const name = path.join(dir, String(index + 1).padStart(2, "0"));
    execFileSync("say", ["-v", record.voice, "-o", `${name}.aiff`, owner.say]);
    execFileSync("ffmpeg", [
      "-loglevel", "error", "-y", "-i", `${name}.aiff`,
      "-ar", "48000", "-ac", "1", "-c:a", "pcm_s16le", `${name}.wav`,
    ]);
    const seconds = Number(execFileSync("ffprobe", [
      "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", `${name}.wav`,
    ], { encoding: "utf8" }));
    if (!(seconds > 0)) throw new Error(`${name}.wav has no length: ${seconds}`);
    owner.speech = { file: "file://" + `${name}.wav`, seconds };
    console.log(`${path.basename(name)}  ${seconds.toFixed(2)} s  ${owner.say}`);
  });
}

async function main() {
  const args = process.argv.slice(2);
  const still = args.indexOf("--still");
  const times = still < 0 ? [] : args.splice(still).slice(1).map(Number);
  const final = args.includes("--final");
  const [work, video] = args.filter(arg => !arg.startsWith("--")).map(arg => path.resolve(arg));
  if (!work || !video) throw new Error("usage: render.cjs <work dir> <out.mp4> [--final] [--still <seconds>...]");

  const record = JSON.parse(fs.readFileSync(path.join(work, "events.json"), "utf8"));
  speak(record, work);
  const timeline = schedule(record);
  fs.writeFileSync(path.join(work, "timeline.json"), JSON.stringify(timeline));
  const frames = fs
    .readdirSync(path.join(work, "frames"))
    .filter(name => /^\d{4}\.png$/.test(name))
    .sort()
    .map(name => "file://" + path.join(work, "frames", name));
  console.log(`${frames.length} app frames, ${timeline.duration.toFixed(2)} s`);

  const browser = await puppeteer.launch({
    executablePath: chrome,
    args: ["--allow-file-access-from-files", "--force-color-profile=srgb", "--hide-scrollbars"],
  });
  const page = await browser.newPage();
  page.on("pageerror", error => console.error(error));
  await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
  await page.goto("file://" + path.join(__dirname, "index.html"));
  await page.evaluate((timeline, frames) => load(timeline, frames), timeline, frames);

  if (times.length > 0) {
    for (const time of times) {
      await page.evaluate(time => renderAt(time), time);
      const name = video.replace(/\.mp4$/, `-${time.toFixed(2)}.png`);
      await page.screenshot({ path: name });
      console.log(name);
    }
    await browser.close();
    return;
  }

  const audio = path.join(work, "audio.wav");
  const sound = await page.evaluate(timeline => renderAudio(timeline), timeline);
  fs.writeFileSync(audio, Buffer.from(sound, "base64"));

  const rate = final ? 4 * timeline.fps : timeline.fps;
  const ffmpeg = spawn(
    "ffmpeg",
    [
      "-loglevel", "error", "-y",
      "-f", "image2pipe", "-framerate", String(rate), "-c:v", "png", "-i", "-",
      "-i", audio,
      "-map", "0:v", "-map", "1:a",
      ...(final ? ["-vf", `tmix=frames=4,fps=${timeline.fps}`] : []),
      "-c:v", "libx264", "-preset", "slow", "-crf", "16", "-pix_fmt", "yuv420p",
      "-af", "loudnorm=I=-14:TP=-1.5:LRA=11", "-ar", "48000", "-c:a", "aac", "-b:a", "192k",
      "-shortest", "-movflags", "+faststart", video,
    ],
    { stdio: ["pipe", "inherit", "inherit"] },
  );
  const total = Math.round(timeline.duration * rate);
  for (let frame = 0; frame < total; frame++) {
    await page.evaluate(time => renderAt(time), frame / rate);
    const png = await page.screenshot({ type: "png", optimizeForSpeed: true });
    if (!ffmpeg.stdin.write(png)) await new Promise(done => ffmpeg.stdin.once("drain", done));
    if (frame % rate === 0) process.stdout.write(`\r${frame / rate} s of ${timeline.duration.toFixed(1)}`);
  }
  ffmpeg.stdin.end();
  const code = await new Promise(done => ffmpeg.on("close", done));
  await browser.close();
  if (code !== 0) throw new Error(`ffmpeg stopped with ${code}`);
  console.log(`\n${video}`);
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
