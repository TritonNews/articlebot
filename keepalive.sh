if ! pgrep -x "articlebot" > /dev/null ; then
    make release
fi