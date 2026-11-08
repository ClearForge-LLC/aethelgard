# S4 pre-probe: can ONE long-lived ngspice (pipe mode) be advanced quantum by
# quantum to a set time, with a source altered between quanta, and resumed?
# ngspice 44.2 pipe mode segfaults on a non-tty stdin (see h4 notes), so it runs under script(1)'s pty.
cd /w
( echo 'source rc.cir'
  echo 'stop when time > 2.5u'
  echo 'tran 1n 22.5u 0 5n'
  echo 'let n=length(time)-1'
  echo 'print time[n] v(line)[n]'
  echo 'echo Q1-END'
  echo 'alter vctl dc = 0'
  echo 'delete all'
  echo 'stop when time > 5u'
  echo 'resume'
  echo 'let n=length(time)-1'
  echo 'print time[n] v(line)[n]'
  echo 'echo Q2-END'
  echo 'delete all'
  echo 'resume'
  echo 'let n=length(time)-1'
  echo 'print time[n] v(line)[n]'
  echo 'meas tran trise TRIG v(line) VAL=0.99 RISE=1 TARG v(line) VAL=2.31 RISE=1'
  echo 'quit' ) | script -qec "ngspice -p -t dumb" /dev/null 2>&1
