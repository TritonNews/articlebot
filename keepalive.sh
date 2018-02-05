if ! pgrep -x "articlebot" > /dev/null ; then
  echo "Restarting bot ..."
  make release
fi