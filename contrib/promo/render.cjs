// Renders the promotional video: index.html drawn frame by frame in a
// headless Chrome, encoded by ffmpeg.
//
//   PROMO_DB=<a SQLite file> cargo test --features shots --lib promo -- --ignored
//   npm install --prefix /tmp/promo puppeteer-core
//   NODE_PATH=/tmp/promo/node_modules node contrib/promo/render.cjs [out.mp4]
//
// With `--still <seconds>...` it writes those frames as PNG beside the
// video's path, for a look before the whole render.

const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const puppeteer = require("puppeteer-core");

const root = path.resolve(__dirname, "../..");
const shots = path.join(root, "target/promo");
const chrome =
  process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

async function main() {
  const args = process.argv.slice(2);
  const still = args.indexOf("--still");
  const times = still < 0 ? [] : args.splice(still).slice(1).map(Number);
  const video = path.resolve(args[0] || path.join(root, "target/tabletist-promo.mp4"));

  const files = fs
    .readdirSync(shots)
    .filter(name => /^\d+-\w+\.png$/.test(name))
    .sort()
    .map(name => "file://" + path.join(shots, name));
  if (files.length === 0) throw new Error(`no frames in ${shots}: run the promo test first`);

  const browser = await puppeteer.launch({
    executablePath: chrome,
    args: ["--allow-file-access-from-files", "--force-color-profile=srgb", "--hide-scrollbars"],
  });
  const page = await browser.newPage();
  await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
  await page.goto("file://" + path.join(__dirname, "index.html"));
  const { duration, fps, end } = await page.evaluate(files => load(files), files);
  console.log(`${files.length} app frames, the last on screen at ${end.toFixed(2)} s`);

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

  const ffmpeg = spawn(
    "ffmpeg",
    [
      "-loglevel", "error", "-y",
      "-f", "image2pipe", "-framerate", String(fps), "-c:v", "png", "-i", "-",
      "-c:v", "libx264", "-preset", "slow", "-crf", "16", "-pix_fmt", "yuv420p",
      "-movflags", "+faststart", video,
    ],
    { stdio: ["pipe", "inherit", "inherit"] },
  );
  const total = Math.round(duration * fps);
  for (let frame = 0; frame < total; frame++) {
    await page.evaluate(time => renderAt(time), frame / fps);
    const png = await page.screenshot({ type: "png", optimizeForSpeed: true });
    if (!ffmpeg.stdin.write(png)) await new Promise(done => ffmpeg.stdin.once("drain", done));
    if (frame % fps === 0) console.log(`${frame / fps} s of ${duration}`);
  }
  ffmpeg.stdin.end();
  await new Promise(done => ffmpeg.on("close", done));
  await browser.close();
  console.log(video);
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
