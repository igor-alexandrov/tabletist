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
import re
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
# The window class to wait for, as a regular expression: Tabletist by
# default, another app with --class.
APP_CLASS = r"dev\.tabletist\.Tabletist"
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
def stage(hypr, app_class=APP_CLASS):
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
        pattern = ("^(" + app_class + ")$").replace("\\", "\\\\")
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
    """The windows ext-foreign-toplevel-list names, oldest first, as dicts
    of `identifier`, `app_id` and `title`.

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
                elif target in handles and opcode in (2, 3, 4):  # title, app_id, identifier
                    field = {2: "title", 3: "app_id", 4: "identifier"}[opcode]
                    handles[target][field] = wayland_string(body, 0)[0]
                elif target == waiting and opcode == 0:  # wl_callback.done
                    if waiting == 5:
                        return [h for h in handles.values() if "identifier" in h]
                    wire.sendall(message(1, 0, struct.pack("<I", 5)))
                    waiting = 5


def capture(hypr, identifier):
    """The window's own picture, as a digest: what the app last drew,
    whatever the compositor shows over it."""
    # grim can wait for a frame forever from a window that is going away
    # (a splash): 2 seconds, then that window counts as gone.
    done = subprocess.run(
        ["grim", "-T", identifier, "-t", "ppm", "-"], env=hypr.env, capture_output=True, check=True, timeout=2
    )
    return hashlib.blake2b(done.stdout, digest_size=16).hexdigest()


def evict(path):
    """Drops a file's pages from the page cache, as far as the kernel lets an
    unprivileged process (pages another process has mapped stay). A folder:
    every file in it, for an app that is more than one file."""
    if os.path.isdir(path):
        for folder, _, names in os.walk(path):
            for name in names:
                full = os.path.join(folder, name)
                if os.path.isfile(full) and not os.path.islink(full):
                    evict(full)
        return
    with open(path, "rb") as file:
        os.posix_fadvise(file.fileno(), 0, 0, os.POSIX_FADV_DONTNEED)


class Launch:
    """One start of the app on the headless output."""

    def __init__(
        self, hypr, command, app_class=APP_CLASS, env=(), fresh_profile=False, seed=None, title=".*"
    ):
        self.hypr = hypr
        self.app_class = app_class
        self.title = title
        self.profiles = set(glob.glob(DEMO_PROFILES))
        environment = dict(hypr.env, **dict(item.split("=", 1) for item in env))
        # A first start in an empty profile: config, data, cache and state of
        # its own, removed when the window closes.
        self.profile = tempfile.mkdtemp(prefix="tabletist-measure-profile-") if fresh_profile else None
        if self.profile:
            for name in ("CONFIG", "DATA", "CACHE", "STATE"):
                folder = os.path.join(self.profile, name.lower())
                os.mkdir(folder)
                environment[f"XDG_{name}_HOME"] = folder
            # Settings the app is to start with, laid out as the profile is
            # (config/, data/, cache/, state/).
            if seed:
                shutil.copytree(seed, self.profile, dirs_exist_ok=True)
        self.before = {window["identifier"] for window in toplevels(hypr.wayland)}
        self.identifier = None
        self.log = tempfile.TemporaryFile()
        self.start = time.monotonic()
        self.process = subprocess.Popen(
            command, env=environment, stdout=self.log, stderr=subprocess.STDOUT, start_new_session=True
        )
        self.address = None
        self.window_at = None
        self.captures = []
        self.settled_on_first = False
        self.xwayland = None
        self.windows = 0

    def matches(self, app_id, title):
        return (
            re.fullmatch(self.app_class, app_id or "") is not None
            and re.fullmatch(self.title, title or "") is not None
        )

    def newest(self):
        """The app's newest window that was not there before the launch."""
        new = [
            window["identifier"]
            for window in toplevels(self.hypr.wayland)
            if window["identifier"] not in self.before
            and self.matches(window.get("app_id"), window.get("title"))
        ]
        return new[-1] if new else None

    def wait_window(self):
        def opened(line):
            name, _, data = line.partition(">>")
            fields = data.split(",", 3)
            return name == "openwindow" and len(fields) == 4 and self.matches(fields[2], fields[3])

        try:
            self.window_at, line = self.hypr.wait_event(opened, self.start)
        except TimeoutError:
            self.log.seek(0)
            sys.stderr.write(self.log.read().decode(errors="replace"))
            raise
        self.address = "0x" + line.partition(">>")[2].split(",")[0]
        self.identifier = self.newest()
        if self.identifier is None:
            raise RuntimeError(f"no {self.app_class} window in the toplevel list")
        self.windows = 1
        return self.window_at - self.start

    def check_xwayland(self):
        """Whether the app drew through XWayland or on Wayland itself, asked
        once the picture settled so the asking costs the timing nothing."""
        self.xwayland = next(
            (c.get("xwayland") for c in self.hypr.json("clients") if c["address"] == self.address), None
        )

    def wait_settled(self):
        """Seconds from the launch to the first frame of the final picture.

        The picture is the newest window's: an app that opens a splash, then
        its main window, then a first-run dialog, settles when the last of
        them holds still and no other window has opened meanwhile."""
        deadline = time.monotonic() + TIMEOUT
        # The window wait_window found is the newest so far.
        checked = time.monotonic()
        while time.monotonic() < deadline:
            if time.monotonic() - checked > 0.1:
                checked = time.monotonic()
                newest = self.newest()
                if newest is not None and newest != self.identifier:
                    self.identifier = newest
                    self.windows += 1
                    self.captures = []
            began = time.monotonic()
            try:
                digest = capture(self.hypr, self.identifier)
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                # The window closed (a splash): look for the next one.
                checked = 0.0
                continue
            self.captures.append((began, digest))
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
        """The window's process: for an AppImage, the app, not the runtime.
        The launched process, when that window is gone (a closed splash)."""
        for client in self.hypr.json("clients"):
            if client["address"] == self.address:
                return client["pid"]
        return self.process.pid

    def send(self, mods, key):
        self.hypr.ctl(
            "dispatch",
            f'hl.dsp.send_shortcut({{ mods = "{mods}", key = "{key}", window = "address:{self.address}" }})',
        )

    def close(self):
        """Closes the window as a person would, so the demo removes its
        profile; kills the process group if it does not go."""
        # Every window of the app, a dialog it opened on its first run too.
        tree = set(descendants(self.process.pid))
        for client in self.hypr.json("clients"):
            if client["pid"] in tree or client["address"] == self.address:
                with contextlib.suppress(subprocess.CalledProcessError):
                    self.hypr.ctl("dispatch", f'hl.dsp.window.close({{ window = "address:{client["address"]}" }})')
        for sent in (None, signal.SIGTERM, signal.SIGKILL):
            if sent is not None:
                with contextlib.suppress(ProcessLookupError):
                    os.killpg(self.process.pid, sent)
            try:
                self.process.wait(10)
                break
            except subprocess.TimeoutExpired:
                pass
        # A killed demo leaves its profile behind: remove the ones this run made.
        for profile in set(glob.glob(DEMO_PROFILES)) - self.profiles:
            shutil.rmtree(profile, ignore_errors=True)
        if self.profile:
            shutil.rmtree(self.profile, ignore_errors=True)


def gpu_fdinfo(pid):
    """The fdinfo of each DRM file the process has open."""
    texts = []
    for fd in glob.glob(f"/proc/{pid}/fd/*"):
        with contextlib.suppress(OSError):
            if os.readlink(fd).startswith("/dev/dri/"):
                with open(f"/proc/{pid}/fdinfo/{os.path.basename(fd)}") as file:
                    texts.append(file.read())
    return texts


def memory_of(pid):
    with open(f"/proc/{pid}/smaps_rollup") as file:
        smaps = parse_smaps_rollup(file.read())
    texts = gpu_fdinfo(pid)
    return {
        "rss": smaps.get("Rss", 0),
        "pss": smaps.get("Pss", 0),
        "private": smaps.get("Private_Clean", 0) + smaps.get("Private_Dirty", 0),
        "gpu": parse_gpu_fdinfo(texts),
    }


def descendants(root):
    """`root` and every process under it, from /proc's parent links."""
    children = {}
    for stat in glob.glob("/proc/[0-9]*/stat"):
        with contextlib.suppress(OSError):
            with open(stat) as file:
                text = file.read()
            # The command name is in parentheses and may hold spaces.
            fields = text[text.rindex(")") + 2 :].split()
            children.setdefault(int(fields[1]), []).append(int(stat.split("/")[2]))
    found, queue = [], [root]
    while queue:
        pid = queue.pop()
        found.append(pid)
        queue.extend(children.get(pid, []))
    return found


def memory_of_tree(root):
    """The memory of `root` and its descendants added up: an Electron app's
    renderer and GPU processes, an AppImage's runtime. RSS counts a shared
    page once per process; PSS divides it, so PSS adds up fairly."""
    total = {"processes": 0, "rss": 0, "pss": 0, "private": 0}
    texts = []
    for pid in descendants(root):
        with contextlib.suppress(OSError):
            one = memory_of(pid)
            total["processes"] += 1
            for key in ("rss", "pss", "private"):
                total[key] += one[key]
            texts.extend(gpu_fdinfo(pid))
    total["gpu"] = parse_gpu_fdinfo(texts)
    return total


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
    with stage(hypr, args.app_class):
        for number in range(args.warmup + args.runs):
            if args.cold:
                for path in evicted:
                    evict(shutil.which(path) or path)
            launch = Launch(hypr, args.command, args.app_class, args.env, args.fresh_profile, args.seed, args.title)
            try:
                window = launch.wait_window()
                settled = launch.wait_settled()
                interval = launch.capture_interval()
                launch.check_xwayland()
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
                        "xwayland": launch.xwayland,
                        "windows": launch.windows,
                        "capture_interval": interval,
                    }
                )
            time.sleep(args.pause)
    report = {
        "what": "startup",
        "cold": args.cold,
        "evicted": evicted if args.cold else [],
        "fresh_profile": args.fresh_profile,
        "seed": args.seed,
        "env": args.env,
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


# What a key step may name for a character Hyprland has no key of that name for.
KEY_NAMES = {" ": "space", ",": "comma"}


def run_steps(launch, steps, settle, sample, shot):
    """Drives an app through `steps`, separated by ';':

    - `wait:S`: S seconds.
    - `type:TEXT`: each character as a key.
    - `settle`: until the picture holds still.
    - `sample:NAME`: a memory sample (after `--idle` seconds).
    - `shot:NAME`: the window to <SHOT>-<NAME>.png.
    - `MODS+key` or `MODS+key*N`: a key, N times (`CTRL+t`, `Down*5`).
    """
    for step in filter(None, (part.strip() for part in steps.split(";"))):
        kind, _, value = step.partition(":")
        if kind == "wait":
            time.sleep(float(value))
        elif kind == "type":
            for char in value:
                launch.send("", KEY_NAMES.get(char, char))
                time.sleep(0.03)
        elif step == "settle":
            settle()
        elif kind == "sample":
            sample(value)
        elif kind == "shot":
            shot(value)
        else:
            key, _, times = step.partition("*")
            mods, _, key = key.rpartition("+")
            for _ in range(int(times or 1)):
                launch.send(mods.replace("+", " "), key)
                time.sleep(0.05)


def run_memory(args):
    """Memory idle on the demo, after opening the 100,000-row table, and
    after going to its last row."""
    hypr = Hyprland()
    steps = []
    with stage(hypr, args.app_class):
        launch = Launch(hypr, args.command, args.app_class, args.env, args.fresh_profile, args.seed, args.title)
        try:
            launch.wait_window()
            launch.wait_settled()

            def sample(name):
                time.sleep(args.idle)
                pid = launch.app_pid()
                steps.append(
                    {"step": name, "pid": pid, **memory_of(pid), "tree": memory_of_tree(launch.process.pid)}
                )
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

            if args.idle_only:
                shot("idle")
                sample("idle after the start")
            elif args.steps:
                run_steps(launch, args.steps, settle, sample, shot)
            else:
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
    print(
        json.dumps(
            {
                "what": "memory",
                **describe(args.command),
                "fresh_profile": args.fresh_profile,
                "seed": args.seed,
                "env": args.env,
                "steps": steps,
            },
            indent=2,
        )
    )


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


def add_app_options(parser):
    parser.add_argument(
        "--class", dest="app_class", default=APP_CLASS, help="the window's class, as a regular expression"
    )
    parser.add_argument(
        "--title", default=".*", help="with --class: the window's title too, as a regular expression"
    )
    parser.add_argument("--env", action="append", default=[], help="NAME=VALUE for the app's environment")
    parser.add_argument(
        "--fresh-profile", action="store_true", help="empty XDG config, data, cache and state for each launch"
    )
    parser.add_argument(
        "--seed", help="with --fresh-profile: a folder copied into each profile first (config/, data/, ...)"
    )


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
    add_app_options(startup)
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
    memory.add_argument(
        "--idle-only", action="store_true", help="one sample idle after the start (any app, no keys)"
    )
    memory.add_argument("--steps", help="keys, waits and samples for another app (see run_steps)")
    add_app_options(memory)
    memory.add_argument("command", nargs="+")
    memory.set_defaults(run=run_memory)

    sizes = commands.add_parser("sizes", help="download and binary sizes of a release")
    sizes.add_argument("tag")
    sizes.add_argument("--repo", default="igor-alexandrov/tabletist")
    sizes.set_defaults(run=run_sizes)

    args = parser.parse_args()
    # A SIGTERM (a `timeout` around the script) runs the clean-up too: the
    # headless output, the window rule and the profiles go.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
    args.run(args)


if __name__ == "__main__":
    main()
