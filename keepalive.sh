if ! pgrep -x "gedit" > /dev/null ; then
    make release
fi