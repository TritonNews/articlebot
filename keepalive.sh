if ! pgrep -x "articlebot" > /dev/null ; then
  echo "Bot died. Resurrecting ..."
  export PATH="$HOME/.cargo/bin:$PATH"
  make release
fi