# S4 control: does Renode's monitor history (~/.config/renode/history, which persists in HOME) slow every
# monitor command as it grows? The same run (route ii, 47k, ioc, 100 us x 100) with an empty history, then
# with 200,000 lines planted in it, then emptied again. Run in the image: repo at /repo, cache at /cache.
cd /repo/spike/2101/lockstep
B=/cache/target-ls2/release/lockstep
r() { $B --renode-setup s4-ii.setup --reads s4.reads --fault-cmd 'sysbus.i2c1.sensor Fault {}' --quantum-ns 100000 --span-ns 10000000 --deck line-47k.cir --spice ioc --out /tmp/h 2>&1 | tail -1 | grep -o 'loop_s=[0-9.]* per_quantum_ms=[0-9.]*'; echo "    history: $(wc -l < $HOME/.config/renode/history) lines, $(du -k $HOME/.config/renode/history | cut -f1) KiB; seq_sha256=$(sha256sum < /tmp/h.seq | cut -c1-16)"; }
export HOME=$(mktemp -d)
echo "== 1 fresh HOME"; r
echo "== 2 same HOME, history as left by run 1"; r
yes 'sysbus ReadDoubleWord 0x20000000' | head -200000 >> $HOME/.config/renode/history
echo "== 3 200,000 lines added"; r
: > $HOME/.config/renode/history
echo "== 4 history emptied"; r
