if ! pgrep -x "articlebot" > /dev/null ; then
  echo "Bot died. Resurrecting ..."
  export PATH="/home/dmhacker/.cargo/bin:$PATH"
  make release
fi