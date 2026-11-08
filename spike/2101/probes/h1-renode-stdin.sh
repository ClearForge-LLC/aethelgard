# hinge 1: monitor commands on a piped stdin, with delays between them
( echo 'mach create'; sleep 2; echo 'machine LoadPlatformDescription @platforms/cpus/stm32h753.repl'; sleep 3;
  echo 'echo "S-A"'; echo 'emulation RunFor "0.000010000"'; echo 'echo "S-B"'; sleep 1;
  echo 'emulation GetTimeSourceInfo'; echo 'echo "S-C"'; sleep 1; echo quit ) | \
  DOTNET_EnableDiagnostics=0 renode --disable-gui --console --plain 2>&1 | while IFS= read -r l; do printf '%s %s\n' "$(date +%s.%N | cut -c1-14)" "$l"; done
