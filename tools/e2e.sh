#!/usr/bin/env bash
# In-game check for FramedBlocks' camo: a framed cube whose block entity stores
# another block and whose client renderer draws that block in the frame's place.
#
# STATUS (2026-11-18): this scenario needs a game built by the launcher with this
# flat SDK package (`horizon-launcher native-build ports/framedblocks --instance <i>
# --trust-native-code`), because a native mod is compiled into `horizon-client`. The
# prepared 26.3 resources and `jq` are needed too. The retired `.hnmod`'s embedded
# content (`.horizon-cache`, `--dev-mod`, the control leg) is gone: the flat package
# carries `assets/` and `data/` inside itself. The last run of this script was on the
# retired bundle (`ports/_shared/evidence/framedblocks/e2e-embedded.log`); the SDK
# migration did not re-run it (no launcher and no game build at `0b14f8a`).
#
# Scene: creative launch (safe teleport), a 5x5 stone platform at y=199 high in
# the Overworld, the frame at (100,200,100), the player at (100,200,102), then
# survival for the item accounting. `--offscreen` so screenshots work.
#
# Checks:
#   1. The placed frame draws its own texture; the central crop has no camo
#      colour. Its blockstate model is empty, so only the block entity renderer
#      can put pixels there.
#   2. `/framedblocks camo 100 200 100` reads `none` from the block entity.
#   3. A right click with a gold block applies the camo, consumes one gold
#      block, the readout names `minecraft:gold_block`, and the client draws
#      the gold block's model instead of the frame.
#   4. Breaking the frame drops the frame and the gold block.
#   5. The framed hammer removes a second camo, the gold block comes back, the
#      readout says `none`, and breaking then drops no gold.
#   6. A frame with no camo drops no camo item (control).
#   7. Save, quit, reopen: the block entity still reads the camo and draws it.
#
# COUNTERFACTUAL=1 repeats the identical scenario with a diamond block and
# requires diamond, not gold: a renderer that ignored the stored camo, or one
# hard-coded to gold, fails one of the two runs.
#
# SELFTEST=1 flips the camo assertion to "minecraft:diamond_block" after a gold
# click, so the run must FAIL: the proof the assertion can fail.
#
# The screenshot check is fail-capable: a block entity renderer that never runs
# leaves the frame invisible (no brown), and one that draws nothing for a camo
# leaves no gold/cyan. The readout is the server's block entity, so a camo that
# only lived in the renderer would still fail.
set -euo pipefail

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/framedblocks
# A launcher-built game carries the mod; the control binary is the campaign's
# shared one when a build at the pinned SDK exists (none at 0b14f8a yet), else
# point CTL at a `horizon-ctl` built from the same build as GAME.
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-0b14f8ac/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/framedblocks-e2e.sock}
COUNTERFACTUAL=${COUNTERFACTUAL:-0}
SKIP_RELOAD=${SKIP_RELOAD:-0}
# SELFTEST=1 asserts the applied camo is a diamond block while gold was clicked,
# so the run must FAIL: the proof the camo assertion can fail.
SELFTEST=${SELFTEST:-0}
# The launcher-built game carries the mod, so no --dev-mod is passed. A game
# built from source without it has neither the block nor the code: point
# DEV_MOD=1 at a content folder only if you are testing the content alone.
DEV_MOD=()
if [ "${DEV_MOD_CONTENT:-0}" = 1 ]; then DEV_MOD=(--dev-mod "$PORT/framedblocks.hmod"); fi

if [ "$COUNTERFACTUAL" = 1 ]; then
    CAMO=minecraft:diamond_block
    PRESENT=cyan
else
    CAMO=minecraft:gold_block
    PRESENT=gold
fi

FX=100; FY=200; FZ=100
PX=100; PY=200; PZ=102

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client (a launcher build with this mod): $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/framedblocks-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
count_item() { ctl inventory | jq -r --arg i "$1" '[.counts[$i] // 0] | add'; }

# `give` is a server command; the client's inventory copy arrives a tick or two
# later. Wait for it before selecting or counting.
wait_item() { # item minimum
    local i=0 n
    n=$(count_item "$1")
    while [ "${n:-0}" -lt "$2" ] && [ "$i" -lt 40 ]; do
        ctl wait 5 >/dev/null
        n=$(count_item "$1")
        i=$((i + 1))
    done
    echo "${n:-0}"
}
select_item() { # item
    local i=0
    while ! ctl select "$1" >/dev/null 2>&1; do
        i=$((i + 1))
        [ "$i" -lt 40 ] || fail "the hotbar never held $1"
        ctl wait 5 >/dev/null
    done
}

# The central 40% x 40% of the frame: the block, without the hand or the HUD.
count_colour() {
    python3 - "$1" "$2" <<'PY'
import sys
from PIL import Image
path, kind = sys.argv[1], sys.argv[2]
image = Image.open(path).convert("RGB")
w, h = image.size
crop = image.crop((w * 3 // 10, h * 3 // 10, w * 7 // 10, h * 7 // 10))
data = crop.tobytes()

def gold(r, g, b):
    return r > 150 and g > 110 and b < 110 and r - b > 80 and g - b > 50

def cyan(r, g, b):
    return r < 140 and g > 120 and b > 120 and g >= b - 10 and b - r > 40 and g - r > 40

def brown(r, g, b):
    return 25 <= r <= 150 and 15 <= g <= 120 and b <= 90 and r >= g >= b and r - b >= 18

match = {"gold": gold, "cyan": cyan, "brown": brown}[kind]
count = 0
for i in range(0, len(data), 3):
    if match(data[i], data[i + 1], data[i + 2]):
        count += 1
print(count)
PY
}

shot() { ctl screenshot "$1" >/dev/null; }
expect_colour() { # file colour minimum message
    local n
    n=$(count_colour "$1" "$2")
    echo "  $2 pixels in $(basename "$1"): $n"
    [ "$n" -ge "$3" ] || fail "$4 (expected at least $3 $2 pixels, saw $n)"
}
expect_no_colour() { # file colour maximum message
    local n
    n=$(count_colour "$1" "$2")
    echo "  $2 pixels in $(basename "$1"): $n"
    [ "$n" -le "$3" ] || fail "$4 (expected at most $3 $2 pixels, saw $n)"
}

# The server's block entity readout for a frame: `/framedblocks camo x y z`.
# `messages` lists newest first, and the reply arrives a tick or two after the
# chat line, so wait for the newest matching line to change.
camo_read() {
    local prev line i=0
    prev=$(ctl messages 40 | jq -r '.chat[]' | grep -F "framedblocks camo $1 $2 $3 =" | head -1 || true)
    ctl chat "/framedblocks camo $1 $2 $3" >/dev/null
    while [ "$i" -lt 40 ]; do
        ctl wait 2 >/dev/null
        line=$(ctl messages 40 | jq -r '.chat[]' | grep -F "framedblocks camo $1 $2 $3 =" | head -1 || true)
        if [ "$line" != "$prev" ]; then
            echo "$line"
            return
        fi
        i=$((i + 1))
    done
    echo "$line"
}

expect_camo() { # pos-x pos-y pos-z expected-substring message
    local line
    line=$(camo_read "$1" "$2" "$3")
    echo "  read: $line"
    case "$line" in
        *"$4"*) ;;
        *) fail "$5 (read '"$line"', expected '$4')" ;;
    esac
}

# The server processes the use packet after the ctl reply, so apply and then
# poll the block entity readout (a retry is a no-op once the camo is set).
apply_camo() { # camo-item
    local attempt line
    for attempt in $(seq 1 10); do
        select_item "$1"
        ctl wait 5 >/dev/null
        local reply
        reply=$(ctl interact "$FX" "$FY" "$FZ")
        echo "  interact: $reply"
        ctl wait 10 >/dev/null
        line=$(camo_read "$FX" "$FY" "$FZ")
        case "$line" in
            *"$1"*) echo "  applied: $line"; return 0 ;;
        esac
    done
    fail "the camo was never stored (last read '$line')"
}

# The hammer removal is server-side too: poll until the readout says none.
expect_removed() { # message
    local attempt line
    for attempt in $(seq 1 10); do
        line=$(camo_read "$FX" "$FY" "$FZ")
        case "$line" in
            *"= none"*) echo "  removed: $line"; return 0 ;;
        esac
        ctl wait 10 >/dev/null
    done
    fail "$1 (last read '$line')"
}

launch() { # launch [WORLD]
    local world=${1:-FramedBlocksE2E}
    rm -f "$SOCK"
    ctl launch \
        --game "$GAME" --world "$world" --seed 20261105 --creative --offscreen --wait \
        --game-dir "$SCRATCH/game" --resources "$RES" "${DEV_MOD[@]}" >/dev/null
    ctl wait-ready 180 >/dev/null
    ctl command "time set 6000" >/dev/null
    ctl tp "$PX" "$PY" "$PZ" >/dev/null
    ctl wait 40 >/dev/null
    ctl fill 98 199 98 102 199 102 stone >/dev/null
    ctl wait 10 >/dev/null
    ctl tp "$PX" "$PY" "$PZ" >/dev/null
    ctl wait 20 >/dev/null
    ctl command "gamemode survival @s" >/dev/null
    ctl wait 5 >/dev/null
}

aim() { ctl look-at block "$FX" "$FY" "$FZ" >/dev/null; ctl wait 5 >/dev/null; }

cleanup() {
    rc=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" || true
    echo "EXIT:$rc"
}
trap cleanup EXIT

if [ "${DEV_MOD_CONTENT:-0}" = 1 ]; then
    echo "== launch (creative scene, survival accounting, offscreen renderer, content .hmod only) =="
else
    echo "== launch (creative scene, survival accounting, offscreen renderer, mod in the launcher build) =="
fi
launch
echo "  state: $(ctl state | jq -c '{pos:.player.position,on_ground:.player.on_ground,screen:.screen}')"

echo "== 1. the frame draws its own texture =="
ctl setblock "$FX" "$FY" "$FZ" framedblocks:framed_cube >/dev/null
ctl wait 20 >/dev/null
aim
BEFORE=$SCRATCH/before.png
shot "$BEFORE"
expect_colour "$BEFORE" brown 1500 "the framed cube drew no frame texture"
expect_no_colour "$BEFORE" "$PRESENT" 300 "the frame already shows the camo colour before any camo was applied"

echo "== 2. the block entity starts without a camo =="
expect_camo "$FX" "$FY" "$FZ" "= none" "the fresh frame's block entity already holds a camo"

echo "== 3. a right click with $CAMO applies the camo =="
ctl give "$CAMO" 4 >/dev/null
wait_item "$CAMO" 1 >/dev/null
before_count=$(count_item "$CAMO")
[ "$before_count" -ge 1 ] || fail "the client never saw the $CAMO the server gave"
apply_camo "$CAMO"
after_count=$(count_item "$CAMO")
echo "  $CAMO before: $before_count, after: $after_count"
[ "$after_count" -eq $((before_count - 1)) ] || fail "applying the camo did not consume one $CAMO"
expect_camo "$FX" "$FY" "$FZ" "$CAMO" "the block entity does not store the applied camo"
if [ "$SELFTEST" = 1 ]; then
    echo "== SELFTEST: the applied camo must read minecraft:diamond_block (it does not) =="
    expect_camo "$FX" "$FY" "$FZ" "minecraft:diamond_block" \
        "selftest: the stored camo was asserted to be a diamond block"
fi
AFTER=$SCRATCH/after.png
shot "$AFTER"
expect_colour "$AFTER" "$PRESENT" 3000 "the camo was not drawn in the frame's place"

echo "== 4. breaking the frame drops the frame and the camo item =="
aim
broken=$(ctl break "$FX" "$FY" "$FZ" --timeout 30)
echo "  break: $broken"
frame_drop=$(echo "$broken" | jq -r '.drops["framedblocks:framed_cube"] // 0')
camo_drop=$(echo "$broken" | jq -r --arg i "$CAMO" '.drops[$i] // 0')
[ "$frame_drop" -ge 1 ] || fail "breaking the frame dropped no framed_cube"
[ "$camo_drop" -ge 1 ] || fail "breaking the camo'd frame dropped no $CAMO"

echo "== 5. the framed hammer removes a camo and gives it back =="
ctl setblock "$FX" "$FY" "$FZ" framedblocks:framed_cube >/dev/null
ctl wait 20 >/dev/null
apply_camo "$CAMO"
gold_before=$(count_item "$CAMO")
ctl give framedblocks:framed_hammer 1 >/dev/null
wait_item framedblocks:framed_hammer 1 >/dev/null
select_item framedblocks:framed_hammer
ctl wait 5 >/dev/null
aim
ctl interact "$FX" "$FY" "$FZ" >/dev/null
ctl wait 20 >/dev/null
expect_removed "the hammer did not clear the camo"
gold_after=$(count_item "$CAMO")
echo "  $CAMO before the hammer: $gold_before, after: $gold_after"
[ "$gold_after" -eq $((gold_before + 1)) ] || fail "the hammer did not give the camo item back"
expect_camo "$FX" "$FY" "$FZ" "= none" "the hammer did not clear the camo"
aim
cleared=$(ctl break "$FX" "$FY" "$FZ" --timeout 30)
cleared_camo=$(echo "$cleared" | jq -r --arg i "$CAMO" '.drops[$i] // 0')
echo "  break after removal: $cleared"
[ "$cleared_camo" -eq 0 ] || fail "a frame whose camo the hammer removed dropped $CAMO anyway"

echo "== 6. a frame with no camo drops no camo item =="
ctl setblock "$FX" "$FY" "$FZ" framedblocks:framed_cube >/dev/null
ctl wait 20 >/dev/null
aim
plain=$(ctl break "$FX" "$FY" "$FZ" --timeout 30)
echo "  break: $plain"
plain_frame=$(echo "$plain" | jq -r '.drops["framedblocks:framed_cube"] // 0')
plain_camo=$(echo "$plain" | jq -r --arg i "$CAMO" '.drops[$i] // 0')
[ "$plain_frame" -ge 1 ] || fail "breaking the plain frame dropped no framed_cube"
[ "$plain_camo" -eq 0 ] || fail "a frame without a camo dropped $CAMO anyway"

if [ "$SKIP_RELOAD" = 1 ]; then
    echo "SKIP_RELOAD=1: the save/reopen leg is skipped"
else
    echo "== 7. the camo survives save and reopen =="
    ctl setblock "$FX" "$FY" "$FZ" framedblocks:framed_cube >/dev/null
    ctl wait 20 >/dev/null
    apply_camo "$CAMO"
    ctl save >/dev/null
    ctl quit >/dev/null
    sleep 3
    launch
    aim
    expect_camo "$FX" "$FY" "$FZ" "$CAMO" "the camo was not stored after reopening the world"
    RELOAD=$SCRATCH/reload.png
    shot "$RELOAD"
    expect_colour "$RELOAD" "$PRESENT" 3000 "the camo was not drawn after reopening the world"
    aim
    reopened=$(ctl break "$FX" "$FY" "$FZ" --timeout 30)
    echo "  break: $reopened"
    reopened_frame=$(echo "$reopened" | jq -r '.drops["framedblocks:framed_cube"] // 0')
    reopened_camo=$(echo "$reopened" | jq -r --arg i "$CAMO" '.drops[$i] // 0')
    [ "$reopened_frame" -ge 1 ] || fail "the reopened frame dropped no framed_cube"
    [ "$reopened_camo" -ge 1 ] || fail "the reopened frame dropped no $CAMO"
fi

ctl quit >/dev/null 2>&1 || true
trap - EXIT
sleep 1
rm -rf "$SCRATCH" || true
if [ "$COUNTERFACTUAL" = 1 ]; then
    echo "COUNTERFACTUAL PASS: the same clicks with a diamond block read and drew diamond, not gold"
elif [ "$SKIP_RELOAD" = 1 ]; then
    echo "PASS (reload leg skipped): the frame drew its texture, the block entity stored the gold camo, the client drew it, the break returned frame + camo, the hammer removed and returned it, and an uncamo'd frame returned no camo"
else
    echo "PASS: the frame drew its texture, the block entity stored the gold camo, the client drew it, the break returned frame + camo, the hammer removed and returned it, an uncamo'd frame returned no camo, and the camo survived save/reopen"
fi
echo "EXIT:0"
