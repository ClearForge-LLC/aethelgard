# S4 timing, re-taken with a fresh HOME per run, as the CLI's sandbox gives every call (HOME = its own workdir).
# Renode appends every monitor command to $HOME/.config/renode/history, and each command slows as the file grows
# (out/s4-history.txt), so the matrix's later runs in one container were inflated; their state hashes are not
# affected (s4-history.txt) and stand. One run per row here. Run in the image: repo at /repo, cache at /cache.
cd /repo/spike/2101/lockstep
B=/cache/target-ls2/release/lockstep; O=/cache/s4t; mkdir -p $O
ii=(--renode-setup s4-ii.setup --fault-cmd 'sysbus.i2c1.sensor Fault {}')
run() { # tag span quantum reads backend R [history-mode]
  local tag=$1 span=$2 q=$3 reads=$4 be=$5 r=$6 hist=${7:-fresh}
  export HOME=$(mktemp -d)
  if [ $hist = devnull ]; then mkdir -p $HOME/.config/renode; ln -s /dev/null $HOME/.config/renode/history; fi
  local deck=(); [ $be != none ] && deck=(--deck line-$r.cir)
  local line; line=$(timeout 900 $B "${ii[@]}" --reads $reads --quantum-ns $q --span-ns $span "${deck[@]}" --spice $be --out $O/$tag 2>&1 | tail -1)
  local h=$HOME/.config/renode/history
  echo "$tag ${line:-KILLED_AT_900s} history_lines=$( [ -L $h ] && echo symlink || wc -l < $h 2>/dev/null)"
}
for r in 47k 2k2; do for be in ioc lib pipe; do for q in 1000000 100000 10000; do
  run t-$be-q$q-$r 10000000 $q s4.reads $be $r
done; done; done
for r in 2k2 47k; do for be in ioc lib none; do run t-$be-q1000-$r-1ms 1000000 1000 s4.reads $be $r; done; done
for be in ioc none; do for q in 100000 10000; do run t-min-$be-q$q-47k 10000000 $q s4-min.reads $be 47k; done; run t-min-$be-q1000-47k-1ms 1000000 1000 s4-min.reads $be 47k; done
run t-ioc-q1000-47k-4ms-fresh 4000000 1000 s4.reads ioc 47k fresh
run t-ioc-q1000-47k-4ms-devnull 4000000 1000 s4.reads ioc 47k devnull
