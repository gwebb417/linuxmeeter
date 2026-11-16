#!/usr/bin/env bash
# Run a command against a private, throwaway PipeWire daemon.
#
# The daemon has no hardware devices and no session manager: just a null sink
# standing in for a sound card. It listens on its own socket inside a temp
# runtime dir, so nothing here can see — or disturb — the user's real audio
# session. XDG_CONFIG_HOME is redirected too, so tests never touch
# ~/.config/linuxmeeter.
#
#   scripts/with-test-daemon.sh cargo test -p lm-engine --test audio -- --ignored
set -euo pipefail

if ! command -v pipewire >/dev/null 2>&1; then
    echo "with-test-daemon: 'pipewire' is not installed; cannot run audio tests" >&2
    exit 127
fi

DIR=$(mktemp -d "${TMPDIR:-/tmp}/linuxmeeter-test.XXXXXX")
DAEMON_PID=""

cleanup() {
    if [ -n "$DAEMON_PID" ] && kill -0 "$DAEMON_PID" 2>/dev/null; then
        kill "$DAEMON_PID" 2>/dev/null || true
        wait "$DAEMON_PID" 2>/dev/null || true
    fi
    rm -rf "$DIR"
}
trap cleanup EXIT INT TERM

mkdir -p "$DIR/run" "$DIR/config/pipewire/pipewire.conf.d"
chmod 700 "$DIR/run"

# Drop-in over the distro's stock pipewire.conf: fixed clock (so metering maths
# are predictable) plus one null sink named alsa_output.* — the engine only
# treats alsa_/bluez_ nodes as real devices.
cat > "$DIR/config/pipewire/pipewire.conf.d/10-linuxmeeter-test.conf" <<'EOF'
context.properties = {
    default.clock.rate        = 48000
    default.clock.quantum     = 1024
    default.clock.min-quantum = 1024
    default.clock.max-quantum = 1024
}
context.objects = [
    { factory = adapter
      args = {
        factory.name     = support.null-audio-sink
        node.name        = "alsa_output.lm-test-sink"
        node.description = "linuxmeeter test sink"
        media.class      = Audio/Sink
        # A fully fixed format, not just a channel map. PipeWire 1.4.x's
        # audioadapter parses the follower's EnumFormat *before* fixating it,
        # so a choice (the null sink's default format/rate ranges) parses as 0
        # and audioconvert rejects the resulting PortConfig with -EINVAL: the
        # sink silently stays in convert mode with one unnamed port.
        audio.format     = F32P
        audio.rate       = 48000
        audio.channels   = 2
        audio.position   = [ FL FR ]
        object.linger    = true
        # No session manager here, so the adapter must configure its own ports.
        adapter.auto-port-config = {
            mode     = dsp
            monitor  = false
            control  = false
            position = preserve
        }
      }
    }
]
EOF

# Client streams (filter-chain capture/playback, meter taps, the test tone)
# can't use the same trick: filter-chain never puts a rate in its EnumFormat,
# so a client-side adapter.auto-port-config fails for it exactly as above and
# its nodes come up with no ports at all. Instead the test rig plays session
# manager for its own lm.* nodes and sets a full DSP PortConfig on each as it
# appears. This tells it to; against a real session (which already does that
# job) it stays unset and the rig keeps out of the way.
export LM_TEST_NO_SESSION_MANAGER=1

XDG_RUNTIME_DIR="$DIR/run" XDG_CONFIG_HOME="$DIR/config" PIPEWIRE_CORE=lm-test \
    pipewire </dev/null >"$DIR/daemon.log" 2>&1 &
DAEMON_PID=$!

# Wait for the socket rather than sleeping a fixed amount.
for _ in $(seq 100); do
    [ -S "$DIR/run/lm-test" ] && break
    sleep 0.05
done
if [ ! -S "$DIR/run/lm-test" ]; then
    echo "with-test-daemon: daemon failed to start" >&2
    cat "$DIR/daemon.log" >&2
    exit 1
fi

export XDG_RUNTIME_DIR="$DIR/run"
export XDG_CONFIG_HOME="$DIR/config"
export PIPEWIRE_REMOTE=lm-test
export LM_TEST_DAEMON=1

set +e
"$@"
status=$?
set -e

# Surface daemon complaints when the command failed — they usually explain why.
if [ $status -ne 0 ] && [ -s "$DIR/daemon.log" ]; then
    echo "--- test daemon log ---" >&2
    tail -30 "$DIR/daemon.log" >&2
fi
exit $status
