# hinge 2: Renode under the CLI's sandbox, monitor commands on a piped stdin.
wd=/tmp/h2wd; mkdir -p $wd; cd $wd
( echo 'mach create'; echo 'machine LoadPlatformDescription @/opt/renode/platforms/cpus/stm32h753.repl';
  echo 'emulation RunFor "0.000010000"'; echo 'echo "S-B"'; echo 'emulation GetTimeSourceInfo'; echo 'echo "S-C"'; echo quit ) | \
  bash /w/sbx.sh $wd /tmp/h2.status -- /usr/local/bin/renode --disable-gui --console --plain 2>&1 | grep -vE 'non existing peripheral'
