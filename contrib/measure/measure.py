#!/usr/bin/env python3
"""Startup time, memory and sizes of Tabletist, measured the same way each time.

How it works and what each number means is in README.md beside this file.
Standard library only. Startup and memory need a running Hyprland session,
`hyprctl` and `grim`: the app opens on a headless output the script adds,
so nothing appears on the screen.

    contrib/measure/measure.py startup --runs 10 -- target/release/tabletist --demo
    contrib/measure/measure.py startup --runs 10 --cold -- ./tabletist-0.5.0-x86_64.AppImage --demo
    contrib/measure/measure.py mount --runs 10 ./tabletist-0.5.0-x86_64.AppImage
    contrib/measure/measure.py memory -- target/release/tabletist --demo
    contrib/measure/measure.py sizes v0.5.0
"""

import argparse
import contextlib
import glob
import hashlib
import json
import os
import selectors
import shutil
import signal
import socket
import statistics
import struct
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
import zipfile

OUTPUT = "TABLETIST-MEASURE"
APP_CLASS = "dev.tabletist.Tabletist"
# The headless output's size: the window fills it, as a tiled window does.
MODE = "1440x900@60"
# How long the picture has to stay the same to count as settled.
SETTLE = 0.5
# How long one launch may take before the run gives up.
TIMEOUT = 30.0
DEMO_PROFILES = os.path.join(tempfile.gettempdir(), "tabletist-demo-*")


# Pure helpers (tested in test_measure.py).


def summary(values):
    """Median, minimum, maximum and count, the way the README reports them."""
    values = sorted(values)
    return {
        "n": len(values),
        "median": statistics.median(values),
        "min": values[0],
        "max": values[-1],
    }


def settled_at(captures, after, hold=SETTLE):
    """When the last picture first appeared, if it has held for `hold` seconds.

    `captures` is a list of (start time, digest) in order. Only captures that
    began after `after` (the window's first frame) count. Returns the start
    of the first capture showing the final picture, or None while it has not
    held long enough.
    """
    captures = [capture for capture in captures if capture[0] >= after]
    if not captures:
        return None
    last = captures[-1][1]
    first = len(captures) - 1
    while first > 0 and captures[first - 1][1] == last:
        first -= 1
    if captures[-1][0] - captures[first][0] < hold:
        return None
    return captures[first][0]


def parse_smaps_rollup(text):
    """The kB fields of /proc/PID/smaps_rollup, as a dict of bytes."""
    fields = {}
    for line in text.splitlines():
        parts = line.split()
        if len(parts) == 3 and parts[2] == "kB":
            fields[parts[0].rstrip(":")] = int(parts[1]) * 1024
    return fields


def parse_gpu_fdinfo(texts):
    """GPU memory a process holds, from the fdinfo of its DRM files.

    One client can show up on several descriptors; each client is counted
    once (by `drm-client-id`). Returns bytes per region ("vram", "gtt", ...).
    """
    clients = {}
    for text in texts:
        fields = {}
        for line in text.splitlines():
            key, _, value = line.partition(":")
            fields[key.strip()] = value.strip()
        client = fields.get("drm-client-id")
        if client is None:
            continue
        regions = {}
        for key, value in fields.items():
            if key.startswith("drm-memory-") and value.endswith(" KiB"):
                regions[key[len("drm-memory-") :]] = int(value.split()[0]) * 1024
        clients[client] = regions
    total = {}
    for regions in clients.values():
        for region, size in regions.items():
            total[region] = total.get(region, 0) + size
    return total


def mib(size):
    return f"{size / 1024 / 1024:.1f} MiB"


# Hyprland.


class Hyprland:
    """The running Hyprland: its control socket and its event socket."""

    def __init__(self):
        runtime = os.environ["XDG_RUNTIME_DIR"]
        signature = os.environ.get("HYPRLAND_INSTANCE_SIGNATURE")
        if not signature:
            found = [
                os.path.basename(os.path.dirname(path))
                for path in glob.glob(os.path.join(runtime, "hypr", "*", ".socket.sock"))
            ]
            if len(found) != 1:
                sys.exit(f"set HYPRLAND_INSTANCE_SIGNATURE: found {len(found)} Hyprland sockets")
            signature = found[0]
        self.env = dict(os.environ, HYPRLAND_INSTANCE_SIGNATURE=signature)
        self.events_path = os.path.join(runtime, "hypr", signature, ".socket2.sock")
        instances = json.loads(self.ctl("instances", "-j"))
        self.wayland = next(
            instance["wl_socket"] for instance in instances if instance["instance"] == signature
        )
        # What the app and grim connect to: Wayland only, no XWayland.
        self.env["WAYLAND_DISPLAY"] = self.wayland
        self.env.pop("DISPLAY", None)
        self.events = []
        self.lock = threading.Condition()
        thread = threading.Thread(target=self._listen, daemon=True)
        thread.start()

    def ctl(self, *args):
        done = subprocess.run(
            ["hyprctl", *args], env=self.env, capture_output=True, text=True, check=True
        )
        return done.stdout

    def eval(self, lua):
        out = self.ctl("eval", lua).strip()
        if out != "ok":
            raise RuntimeError(f"hyprctl eval {lua!r}: {out}")

    def json(self, what):
        return json.loads(self.ctl(what, "-j"))

    def _listen(self):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as events:
            events.connect(self.events_path)
            buffer = b""
            while True:
                chunk = events.recv(65536)
                now = time.monotonic()
                if not chunk:
                    return
                buffer += chunk
                *lines, buffer = buffer.split(b"\n")
                with self.lock:
                    for line in lines:
                        self.events.append((now, line.decode(errors="replace")))
                    self.lock.notify_all()

    def wait_event(self, matches, since, timeout=TIMEOUT):
        """The first event after `since` that `matches`, as (time, line)."""
        deadline = time.monotonic() + timeout
        with self.lock:
            while True:
                for when, line in self.events:
                    if when >= since and matches(line):
                        return when, line
                left = deadline - time.monotonic()
                if left <= 0:
                    raise TimeoutError("no window opened")
                self.lock.wait(left)


@contextlib.contextmanager
def stage(hypr):
    """A headless output far from the real ones, and a rule sending the
    app's windows to it without taking the keyboard. Both go on exit."""
    if any(monitor["name"] == OUTPUT for monitor in hypr.json("monitors")):
        sys.exit(f"{OUTPUT} exists: a run is going, or one was stopped (hyprctl output remove {OUTPUT})")
    hypr.ctl("output", "create", "headless", OUTPUT)
    try:
        # Far away and touching nothing: the pointer cannot wander onto it.
        hypr.eval(
            f'hl.monitor({{ output = "{OUTPUT}", mode = "{MODE}", '
            f'position = "-20000x-20000", scale = 1 }})'
        )
        time.sleep(0.5)
        monitor = next(m for m in hypr.json("monitors") if m["name"] == OUTPUT)
        workspace = monitor["activeWorkspace"]["id"]
        pattern = "^" + APP_CLASS.replace(".", "\\\\.") + "$"
        hypr.eval(
            "tabletist_measure_rule = hl.window_rule({ match = { class = "
            f'"{pattern}" }}, '
            # No open animation: the picture changes only when the app's does.
            f'workspace = "{workspace} silent", no_initial_focus = true, no_anim = true }})'
        )
        try:
            yield
        finally:
            hypr.eval(
                "if tabletist_measure_rule then tabletist_measure_rule:set_enabled(false) end; "
                "tabletist_measure_rule = nil"
            )
    finally:
        hypr.ctl("output", "remove", OUTPUT)


def wayland_string(data, at):
    """A Wayland wire string at `at`: its text and where the next argument starts."""
    (length,) = struct.unpack_from("<I", data, at)
    text = data[at + 4 : at + 4 + length - 1].decode(errors="replace")
    return text, at + 4 + (length + 3) // 4 * 4


def toplevels(wayland):
    """The windows ext-foreign-toplevel-list names, as {identifier: app_id}.

    A Wayland client in a few lines of the wire protocol: get the registry,
    bind the list, and read until two round trips are done.
    """
    path = os.path.join(os.environ["XDG_RUNTIME_DIR"], wayland)

    def message(target, opcode, payload=b""):
        return struct.pack("<II", target, (8 + len(payload)) << 16 | opcode) + payload

    def string(text):
        raw = text.encode() + b"\0"
        return struct.pack("<I", len(raw)) + raw + b"\0" * (-len(raw) % 4)

    interface = "ext_foreign_toplevel_list_v1"
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as wire:
        wire.connect(path)
        # wl_display.get_registry (2), then wl_display.sync (3).
        wire.sendall(message(1, 1, struct.pack("<I", 2)) + message(1, 0, struct.pack("<I", 3)))
        buffer, handles, waiting = b"", {}, 3
        while True:
            chunk = wire.recv(65536)
            if not chunk:
                raise RuntimeError("the compositor closed the connection")
            buffer += chunk
            while len(buffer) >= 8:
                target, word = struct.unpack_from("<II", buffer)
                size, opcode = word >> 16, word & 0xFFFF
                if len(buffer) < size:
                    break
                body, buffer = buffer[8:size], buffer[size:]
                if target == 2 and opcode == 0:  # wl_registry.global
                    (name,) = struct.unpack_from("<I", body)
                    found, at = wayland_string(body, 4)
                    if found == interface:
                        # wl_registry.bind the list as 4, then sync (5).
                        wire.sendall(
                            message(2, 0, struct.pack("<I", name) + string(interface) + struct.pack("<II", 1, 4))
                        )
                elif target == 4 and opcode == 0:  # a toplevel's handle
                    (handle,) = struct.unpack_from("<I", body)
                    handles[handle] = {}
                elif target in handles and opcode in (3, 4):  # app_id, identifier
                    handles[target]["app_id" if opcode == 3 else "identifier"] = wayland_string(body, 0)[0]
                elif target == waiting and opcode == 0:  # wl_callback.done
                    if waiting == 5:
                        return {h["identifier"]: h.get("app_id") for h in handles.values() if "identifier" in h}
                    wire.sendall(message(1, 0, struct.pack("<I", 5)))
                    waiting = 5


def capture(hypr, identifier):
    """The window's own picture, as a digest: what the app last drew,
    whatever the compositor shows over it."""
    done = subprocess.run(
        ["grim", "-T", identifier, "-t", "ppm", "-"], env=hypr.env, capture_output=True, check=True
    )
    return hashlib.blake2b(done.stdout, digest_size=16).hexdigest()


def evict(path):
    """Drops a file's pages from the page cache, as far as the kernel lets an
    unprivileged process (pages another process has mapped stay)."""
    with open(path, "rb") as file:
        os.posix_fadvise(file.fileno(), 0, 0, os.POSIX_FADV_DONTNEED)


class Launch:
    """One start of the app on the headless output."""

    def __init__(self, hypr, command):
        self.hypr = hypr
        self.profiles = set(glob.glob(DEMO_PROFILES))
        self.before = set(toplevels(hypr.wayland))
        self.identifier = None
        self.log = tempfile.TemporaryFile()
        self.start = time.monotonic()
        self.process = subprocess.Popen(
            command, env=hypr.env, stdout=self.log, stderr=subprocess.STDOUT, start_new_session=True
        )
        self.address = None
        self.window_at = None
        self.captures = []
        self.settled_on_first = False

    def wait_window(self):
        def opened(line):
            name, _, data = line.partition(">>")
            return name == "openwindow" and data.split(",")[2] == APP_CLASS

        try:
            self.window_at, line = self.hypr.wait_event(opened, self.start)
        except TimeoutError:
            self.log.seek(0)
            sys.stderr.write(self.log.read().decode(errors="replace"))
            raise
        self.address = "0x" + line.partition(">>")[2].split(",")[0]
        new = [
            identifier
            for identifier, app in toplevels(self.hypr.wayland).items()
            if app == APP_CLASS and identifier not in self.before
        ]
        if len(new) != 1:
            raise RuntimeError(f"expected one new {APP_CLASS} window, found {len(new)}")
        self.identifier = new[0]
        return self.window_at - self.start

    def wait_settled(self):
        """Seconds from the launch to the first frame of the final picture."""
        deadline = time.monotonic() + TIMEOUT
        while time.monotonic() < deadline:
            began = time.monotonic()
            self.captures.append((began, capture(self.hypr, self.identifier)))
            settled = settled_at(self.captures, self.window_at)
            if settled is not None:
                # When the first capture already shows the final picture,
                # the window settled at that moment or earlier.
                self.settled_on_first = settled == self.captures[0][0]
                return settled - self.start
        raise TimeoutError("the window never settled")

    def capture_interval(self):
        starts = [when for when, _ in self.captures]
        gaps = [later - earlier for earlier, later in zip(starts, starts[1:])]
        return statistics.median(gaps) if gaps else 0.0

    def app_pid(self):
        """The window's process: for an AppImage, the app, not the runtime."""
        for client in self.hypr.json("clients"):
            if client["address"] == self.address:
                return client["pid"]
        raise RuntimeError("the window is gone")

    def send(self, mods, key):
        self.hypr.ctl(
            "dispatch",
            f'hl.dsp.send_shortcut({{ mods = "{mods}", key = "{key}", window = "address:{self.address}" }})',
        )

    def close(self):
        """Closes the window as a person would, so the demo removes its
        profile; kills the process group if it does not go."""
        if self.address:
            with contextlib.suppress(subprocess.CalledProcessError):
                self.hypr.ctl("dispatch", f'hl.dsp.window.close({{ window = "address:{self.address}" }})')
        try:
            self.process.wait(10)
        except subprocess.TimeoutExpired:
            os.killpg(self.process.pid, signal.SIGTERM)
            self.process.wait(10)
        # A killed demo leaves its profile behind: remove the ones this run made.
        for profile in set(glob.glob(DEMO_PROFILES)) - self.profiles:
            shutil.rmtree(profile, ignore_errors=True)


def memory_of(pid):
    with open(f"/proc/{pid}/smaps_rollup") as file:
        smaps = parse_smaps_rollup(file.read())
    texts = []
    for fd in glob.glob(f"/proc/{pid}/fd/*"):
        with contextlib.suppress(OSError):
            if os.readlink(fd).startswith("/dev/dri/"):
                with open(f"/proc/{pid}/fdinfo/{os.path.basename(fd)}") as file:
                    texts.append(file.read())
    return {
        "rss": smaps.get("Rss", 0),
        "pss": smaps.get("Pss", 0),
        "private": smaps.get("Private_Clean", 0) + smaps.get("Private_Dirty", 0),
        "gpu": parse_gpu_fdinfo(texts),
    }


def describe(command):
    path = shutil.which(command[0]) or command[0]
    with open(path, "rb") as file:
        digest = hashlib.sha256(file.read()).hexdigest()
    return {"command": command, "path": os.path.abspath(path), "sha256": digest}


# Commands.


def run_startup(args):
    hypr = Hyprland()
    evicted = args.evict or [args.command[0]]
    runs = []
    with stage(hypr):
        for number in range(args.warmup + args.runs):
            if args.cold:
                for path in evicted:
                    evict(shutil.which(path) or path)
            launch = Launch(hypr, args.command)
            try:
                window = launch.wait_window()
                settled = launch.wait_settled()
                interval = launch.capture_interval()
                if args.shot and number == args.warmup + args.runs - 1:
                    subprocess.run(["grim", "-T", launch.identifier, args.shot], env=hypr.env, check=True)
            finally:
                launch.close()
            warm_up = number < args.warmup
            print(
                f"run {number + 1}{' (warm-up, not counted)' if warm_up else ''}: "
                f"window {window * 1000:.0f} ms, settled {settled * 1000:.0f} ms "
                f"{'(the first capture) ' if launch.settled_on_first else ''}"
                f"(captures every {interval * 1000:.0f} ms)",
                file=sys.stderr,
            )
            if not warm_up:
                runs.append(
                    {
                        "window": window,
                        "settled": settled,
                        "settled_on_first_capture": launch.settled_on_first,
                        "capture_interval": interval,
                    }
                )
            time.sleep(args.pause)
    report = {
        "what": "startup",
        "cold": args.cold,
        "evicted": evicted if args.cold else [],
        **describe(args.command),
        "window": summary([run["window"] for run in runs]),
        "settled": summary([run["settled"] for run in runs]),
        "capture_interval": statistics.median(run["capture_interval"] for run in runs),
        "runs": runs,
    }
    print(json.dumps(report, indent=2))


def stop(process):
    """Asks the AppImage runtime to unmount and go (SIGINT), then makes it."""
    for sent in (signal.SIGINT, signal.SIGTERM, signal.SIGKILL):
        with contextlib.suppress(ProcessLookupError):
            os.killpg(process.pid, sent)
        try:
            process.wait(10)
            return
        except subprocess.TimeoutExpired:
            pass


def run_mount(args):
    """How long the AppImage runtime takes to mount the image: from the
    launch to the mount point it prints with --appimage-mount."""
    times = []
    for number in range(args.warmup + args.runs):
        if args.cold:
            evict(args.appimage)
        start = time.monotonic()
        process = subprocess.Popen(
            [args.appimage, "--appimage-mount"],
            stdout=subprocess.PIPE,
            text=True,
            start_new_session=True,
            # A background shell can hand down SIGINT ignored; the runtime
            # needs it to unmount.
            restore_signals=True,
        )
        try:
            # The mount point, or nothing if the runtime stalls.
            with selectors.DefaultSelector() as ready:
                ready.register(process.stdout, selectors.EVENT_READ)
                line = process.stdout.readline() if ready.select(TIMEOUT) else ""
            took = time.monotonic() - start
        finally:
            stop(process)
        if not line.strip():
            sys.exit("the runtime printed no mount point")
        warm_up = number < args.warmup
        print(f"run {number + 1}{' (warm-up)' if warm_up else ''}: {took * 1000:.1f} ms", file=sys.stderr)
        if not warm_up:
            times.append(took)
        time.sleep(args.pause)
    report = {"what": "mount", "cold": args.cold, **describe([args.appimage]), "mount": summary(times)}
    print(json.dumps(report, indent=2))


def run_memory(args):
    """Memory idle on the demo, after opening the 100,000-row table, and
    after going to its last row."""
    hypr = Hyprland()
    steps = []
    with stage(hypr):
        launch = Launch(hypr, args.command)
        try:
            launch.wait_window()
            launch.wait_settled()

            def sample(name):
                time.sleep(args.idle)
                pid = launch.app_pid()
                steps.append({"step": name, "pid": pid, **memory_of(pid)})
                print(f"{name}: {json.dumps(steps[-1]['rss'])} bytes RSS", file=sys.stderr)

            def settle():
                launch.captures = []
                launch.window_at = time.monotonic()
                launch.wait_settled()

            def shot(name):
                if args.shot:
                    subprocess.run(
                        ["grim", "-T", launch.identifier, f"{args.shot}-{name}.png"], env=hypr.env, check=True
                    )

            sample("idle on the demo (users table)")
            # Quick open, the table's name, Enter.
            launch.send("CTRL", "p")
            time.sleep(0.3)
            for letter in args.table:
                launch.send("", letter)
            time.sleep(0.3)
            launch.send("", "Return")
            settle()
            shot("opened")
            sample(f"{args.table} opened (rows 1 to 1,000)")
            # Page after page (Ctrl+Alt+Right, a page of 1,000 rows each) to
            # the table's last; the largest RSS on the way is kept too.
            pid = launch.app_pid()
            peak = 0
            for _ in range(args.pages):
                launch.send("CTRL ALT", "Right")
                time.sleep(args.page_pause)
                peak = max(peak, memory_of(pid)["rss"])
            settle()
            shot("last-page")
            sample(f"{args.table} after {args.pages} pages forward")
            steps[-1]["peak_rss_while_paging"] = peak
        finally:
            launch.close()
    print(json.dumps({"what": "memory", **describe(args.command), "steps": steps}, indent=2))


def run_sizes(args):
    """Each release asset's download size, and the size of the binary inside
    the archives that can be read here (tar.gz and zip)."""
    with tempfile.TemporaryDirectory() as folder:
        subprocess.run(
            ["gh", "release", "download", args.tag, "--repo", args.repo, "--dir", folder], check=True
        )
        assets = []
        for name in sorted(os.listdir(folder)):
            path = os.path.join(folder, name)
            asset = {"asset": name, "download": os.path.getsize(path)}
            if name.endswith(".tar.gz") and "-source" not in name:
                with tarfile.open(path) as archive:
                    member = next(m for m in archive.getmembers() if m.name.endswith("/tabletist"))
                    asset["binary"] = member.size
            elif name.endswith(".zip"):
                with zipfile.ZipFile(path) as archive:
                    member = next(m for m in archive.infolist() if m.filename.endswith("tabletist.exe"))
                    asset["binary"] = member.file_size
            assets.append(asset)
    for asset in assets:
        binary = f"  binary {mib(asset['binary'])}" if "binary" in asset else ""
        print(f"{asset['asset']}: {mib(asset['download'])} ({asset['download']} bytes){binary}", file=sys.stderr)
    print(json.dumps({"what": "sizes", "tag": args.tag, "assets": assets}, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="what", required=True)

    startup = commands.add_parser("startup", help="launch to first frame and to the settled picture")
    startup.add_argument("--runs", type=int, default=10)
    startup.add_argument("--warmup", type=int, default=1, help="runs before the counted ones")
    startup.add_argument("--cold", action="store_true", help="evict the app's file before each run")
    startup.add_argument("--evict", action="append", help="a file to evict (default: the command)")
    startup.add_argument("--pause", type=float, default=1.0, help="seconds between runs")
    startup.add_argument("--shot", help="save the last run's settled picture to this PNG")
    startup.add_argument("command", nargs="+")
    startup.set_defaults(run=run_startup)

    mount = commands.add_parser("mount", help="how long an AppImage takes to mount")
    mount.add_argument("--runs", type=int, default=10)
    mount.add_argument("--warmup", type=int, default=1)
    mount.add_argument("--cold", action="store_true")
    mount.add_argument("--pause", type=float, default=0.5)
    mount.add_argument("appimage")
    mount.set_defaults(run=run_mount)

    memory = commands.add_parser("memory", help="RSS, PSS and GPU memory through the demo")
    memory.add_argument("--table", default="big", help="the demo's 100,000-row table")
    memory.add_argument("--pages", type=int, default=99, help="pages forward (99 reaches row 100,000)")
    memory.add_argument("--page-pause", type=float, default=0.2, help="seconds between pages")
    memory.add_argument("--idle", type=float, default=2.0, help="seconds to wait before a sample")
    memory.add_argument("--shot", help="save the output at each step as <SHOT>-<step>.png")
    memory.add_argument("command", nargs="+")
    memory.set_defaults(run=run_memory)

    sizes = commands.add_parser("sizes", help="download and binary sizes of a release")
    sizes.add_argument("tag")
    sizes.add_argument("--repo", default="igor-alexandrov/tabletist")
    sizes.set_defaults(run=run_sizes)

    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
