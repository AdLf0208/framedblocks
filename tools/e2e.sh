#!/usr/bin/env bash
# In-game check for FramedBlocks' camo: a framed cube whose block entity stores
# another block and whose client renderer draws that block in the frame's place.
#
# STATUS (2026-11-21): this scenario needs a game built by the launcher with this
# flat SDK package (`ports/tools/launcher-game.sh run framedblocks` does the build and
# the run), because a native mod is compiled into `horizon-client`. The prepared 26.3
# resources and `jq` are needed too. The flat package carries `assets/` and `data/`
# inside itself; `CONTROL_GAME` (the same Nightly without the package) is the control
# leg, because nothing of this mod then exists in the game.
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
#   7. `/framedblocks camo <x> <y> <z> <state>` stores a **state**: a grass block camo
#      reads `minecraft:grass_block[snowy=false]` and draws no snow, the same block
#      with `snowy=true` reads that state and draws snow, a property the block does not
#      have is refused without touching the stored camo, and a break drops the state's
#      item. The snow is the proof the renderer uses the stored properties: the two
#      screenshots are the same scene and differ only in the state's `snowy`.
#   8. The carried recipes craft: planks + sticks in the frame pattern give 4 frames
#      (and consume them), then frames + sticks in the hammer pattern give a hammer.
#   9. Save, quit, reopen: the block entity still reads the camo and draws it.
#
# COUNTERFACTUAL=1 repeats the identical scenario with a diamond block and
# requires diamond, not gold: a renderer that ignored the stored camo, or one
# hard-coded to gold, fails one of the two runs.
#
# SELFTEST=1 flips the camo assertion to "minecraft:diamond_block" after a gold
# click, so the run must FAIL: the proof the assertion can fail.
# SELFTEST_STATE=1 asserts the stored state is `snowy=false` right after it was set to
# `snowy=true`, so the run must FAIL: the proof the state assertion can fail.
# SELFTEST_RECIPE=1 asserts the frame grid's result is a framed hammer, so the run must
# FAIL: the proof the recipe assertions read the grid's real result.
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
# SELFTEST_STATE=1 asserts the frame's state is the *default* one right after the command
# stored `snowy=true`, so the run must FAIL: the proof the state assertion can fail.
SELFTEST_STATE=${SELFTEST_STATE:-0}
# SELFTEST_RECIPE=1 asserts the frame pattern's result is a framed hammer, so the run must
# FAIL: the proof the recipe assertions read the grid's real result.
SELFTEST_RECIPE=${SELFTEST_RECIPE:-0}
# KEEP=1 keeps the scratch folder (screenshots and the world) after the run, for a leg
# that has to be looked at rather than read.
KEEP=${KEEP:-0}
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

# Snow over dirt: the pale, slightly cool band `grass_block_snow` draws, which the
# plain grass side has none of. `white` would count the survival HUD's hearts and the
# chat text too, and those are in both shots.
def snow(r, g, b):
    return b >= r and g >= r + 5 and g > 165 and b > 165

match = {"gold": gold, "cyan": cyan, "brown": brown, "snow": snow}[kind]
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

# The same scene with and without a state's property: the difference in `colour`
# pixels is what that property draws. A constant background (sky, platform, hand)
# cancels out, so this is the differential a renderer ignoring the stored state fails.
expect_more_colour() { # more less colour delta message
    local more less
    more=$(count_colour "$1" "$3")
    less=$(count_colour "$2" "$3")
    echo "  $3 pixels: $more with the state, $less without"
    [ $((more - less)) -ge "$4" ] || fail "$5 (expected $4 more $3 pixels, saw $((more - less)))"
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

# The open inventory/crafting menu: the slot holding `item`, or -1.
menu_slot() {
    ctl screen | jq -r --arg i "$1" '[.slots[] | select(.item == $i) | .slot] | first // -1'
}

# The crafting result slot's stack, as `item count`.
result_stack() {
    ctl screen | jq -r '.slots[] | select(.slot == 200) | "\(.item) \(.count)"'
}

# A free inventory slot of the open menu (9 to 35: the menu's player inventory).
free_menu_slot() {
    ctl screen | jq -r '[.slots[] | select(.item == null and .slot >= 9 and .slot <= 35) | .slot] | first // -1'
}

# One crafting-table grid: pick the stack `item` up and right-click one into each
# grid slot given (100 to 108, row-major, 200 the result). The stack given must be
# exactly the number of slots, so nothing is left on the cursor.
fill_grid() { # item slot...
    local item=$1 slot
    shift
    local from
    from=$(menu_slot "$item")
    [ "$from" -ge 0 ] || fail "$item never reached the crafting menu"
    ctl click-slot "$from" >/dev/null
    for slot in "$@"; do ctl click-slot "$slot" right >/dev/null; done
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
    if [ "$KEEP" = 1 ]; then
        echo "  kept: $SCRATCH"
    else
        rm -rf "$SCRATCH" || true
    fi
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

echo "== 7. the camo is a block state and the client draws its properties =="
ctl setblock "$FX" "$FY" "$FZ" framedblocks:framed_cube >/dev/null
ctl wait 20 >/dev/null
# The default state of a grass block has no snow: its side is dirt and grass, no white.
ctl chat "/framedblocks camo $FX $FY $FZ minecraft:grass_block" >/dev/null
ctl wait 10 >/dev/null
expect_camo "$FX" "$FY" "$FZ" "minecraft:grass_block[snowy=false]" \
    "the command did not store the block's default state"
if [ "$SELFTEST_STATE" = 1 ]; then
    echo "== SELFTEST_STATE: the stored state must read minecraft:grass_block[snowy=true] (it does not) =="
    expect_camo "$FX" "$FY" "$FZ" "minecraft:grass_block[snowy=true]" \
        "selftest: the stored state was asserted to be snowy=true after snowy=false"
fi
aim
PLAIN_STATE=$SCRATCH/state-plain.png
shot "$PLAIN_STATE"
# The snow band is measured in the two shots of the same scene: the plain state has 153
# "snow" pixels, the snowy one 20,840 (2026-11-21), so the state is what is being drawn.
expect_no_colour "$PLAIN_STATE" snow 1200 "a grass camo with snowy=false already drew snow"
# `/setblock`'s own state syntax: the same block, one property changed.
ctl chat "/framedblocks camo $FX $FY $FZ minecraft:grass_block[snowy=true]" >/dev/null
ctl wait 10 >/dev/null
expect_camo "$FX" "$FY" "$FZ" "minecraft:grass_block[snowy=true]" \
    "the command did not store the state's property"
aim
SNOWY=$SCRATCH/state-snowy.png
shot "$SNOWY"
expect_colour "$SNOWY" snow 8000 "the snowy state was not drawn at all"
expect_more_colour "$SNOWY" "$PLAIN_STATE" snow 6000 \
    "the renderer ignored the state's properties: snowy=true drew no more snow than snowy=false"
# A property the block does not have is refused, and the stored camo is untouched.
ctl chat "/framedblocks camo $FX $FY $FZ minecraft:stone[nope=1]" >/dev/null
ctl wait 10 >/dev/null
refusal=$(ctl messages 40 | jq -r '.chat[]' | grep -F "is not a block state" | head -1 || true)
echo "  refusal: $refusal"
[ -n "$refusal" ] || fail "a property the block does not have was not refused"
expect_camo "$FX" "$FY" "$FZ" "minecraft:grass_block[snowy=true]" \
    "a refused state replaced the stored camo"
# The drop is the state's own item, not a phantom of the property.
aim
state_break=$(ctl break "$FX" "$FY" "$FZ" --timeout 30)
echo "  break: $state_break"
state_frame=$(echo "$state_break" | jq -r '.drops["framedblocks:framed_cube"] // 0')
state_camo=$(echo "$state_break" | jq -r '.drops["minecraft:grass_block"] // 0')
[ "$state_frame" -ge 1 ] || fail "breaking a state-camo'd frame dropped no framed_cube"
[ "$state_camo" -ge 1 ] || fail "breaking a state-camo'd frame dropped no minecraft:grass_block"

echo "== 8. the carried recipes craft (frame, then hammer) =="
# Both are 3x3 shaped recipes, so a crafting table: its grid is slots 100-108 and its
# result 200. The grid is filled one item at a time from a cursor stack, so each `give`
# is exactly the number of slots that recipe uses and the cursor is empty afterwards.
ctl give minecraft:crafting_table 1 >/dev/null
wait_item minecraft:crafting_table 1 >/dev/null
select_item minecraft:crafting_table
ctl place 101 200 102 >/dev/null
ctl wait 10 >/dev/null
ctl interact 101 200 102 >/dev/null
ctl wait 10 >/dev/null
[ "$(ctl screen | jq -r .craft_size)" = 3 ] || fail "the crafting table did not open its 3x3 grid"
planks_before=$(count_item minecraft:oak_planks)
sticks_before=$(count_item minecraft:stick)
frames_before=$(count_item framedblocks:framed_cube)
# `PSP / S S / PSP`: P at the four corners, S at the four sides.
ctl give minecraft:oak_planks 4 >/dev/null
ctl wait 10 >/dev/null
ctl give minecraft:stick 4 >/dev/null
ctl wait 10 >/dev/null
fill_grid minecraft:oak_planks 100 102 106 108
fill_grid minecraft:stick 101 103 105 107
echo "  frame grid result: $(result_stack)"
if [ "$SELFTEST_RECIPE" = 1 ]; then
    echo "== SELFTEST_RECIPE: the frame grid must give a framed_hammer (it does not) =="
    [ "$(result_stack)" = "framedblocks:framed_hammer 1" ] || fail "selftest: the frame grid was asserted to give a framed hammer"
fi
[ "$(result_stack)" = "framedblocks:framed_cube 4" ] || fail "the frame recipe did not give 4 framed_cube"
ctl click-slot 200 shift >/dev/null
ctl wait 10 >/dev/null
frames_after=$(count_item framedblocks:framed_cube)
planks_after=$(count_item minecraft:oak_planks)
sticks_after=$(count_item minecraft:stick)
echo "  framed_cube: $frames_before -> $frames_after; planks $planks_before -> $planks_after; sticks $sticks_before -> $sticks_after"
[ "$frames_after" -eq $((frames_before + 4)) ] || fail "crafting the frame recipe did not add 4 framed_cube"
[ "$planks_after" -eq "$planks_before" ] || fail "the frame recipe did not consume its planks"
[ "$sticks_after" -eq "$sticks_before" ] || fail "the frame recipe did not consume its sticks"
# ` F / SF / S`: F above and right of the middle, S in the middle and bottom left.
hammer_before=$(count_item framedblocks:framed_hammer)
ctl give minecraft:stick 2 >/dev/null
ctl wait 10 >/dev/null
fill_grid minecraft:stick 104 106
ctl click-slot "$(menu_slot framedblocks:framed_cube)" >/dev/null
for slot in 101 105; do ctl click-slot "$slot" right >/dev/null; done
# Two frames are left on the cursor; the menu's free slot takes them back.
ctl click-slot "$(free_menu_slot)" shift >/dev/null
ctl wait 5 >/dev/null
echo "  hammer grid result: $(result_stack)"
[ "$(result_stack)" = "framedblocks:framed_hammer 1" ] || fail "the hammer recipe did not give a framed_hammer"
ctl click-slot 200 shift >/dev/null
ctl wait 10 >/dev/null
ctl close >/dev/null
ctl wait 5 >/dev/null
hammer_after=$(count_item framedblocks:framed_hammer)
echo "  framed_hammer: $hammer_before -> $hammer_after; sticks left $(count_item minecraft:stick)"
[ "$hammer_after" -eq $((hammer_before + 1)) ] || fail "crafting the hammer recipe did not add a framed_hammer"
[ "$(count_item minecraft:stick)" -eq "$sticks_before" ] || fail "the hammer recipe did not consume its sticks"
[ "$(count_item framedblocks:framed_cube)" -eq $((frames_after - 2)) ] || fail "the hammer recipe did not consume two frames"

if [ "$SKIP_RELOAD" = 1 ]; then
    echo "SKIP_RELOAD=1: the save/reopen leg is skipped"
else
    echo "== 9. the camo survives save and reopen =="
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
if [ "$KEEP" = 1 ]; then
    echo "  kept: $SCRATCH"
else
    rm -rf "$SCRATCH" || true
fi
if [ "$COUNTERFACTUAL" = 1 ]; then
    echo "COUNTERFACTUAL PASS: the same clicks with a diamond block read and drew diamond, not gold"
elif [ "$SKIP_RELOAD" = 1 ]; then
    echo "PASS (reload leg skipped): the frame drew its texture, the block entity stored the gold camo, the client drew it, the break returned frame + camo, the hammer removed and returned it, an uncamo'd frame returned no camo, a command state (snowy=true) read and drew snow that snowy=false did not, and both carried recipes crafted"
else
    echo "PASS: the frame drew its texture, the block entity stored the gold camo, the client drew it, the break returned frame + camo, the hammer removed and returned it, an uncamo'd frame returned no camo, a command state (snowy=true) read and drew snow that snowy=false did not, both carried recipes crafted with their ingredients consumed, and the camo survived save/reopen"
fi
echo "EXIT:0"
