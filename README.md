# majjit

A [Jujutsu](https://github.com/jj-vcs/jj) TUI inspired by [Magit](https://magit.vc/)!

Very much a work in progress, much more to come. But I already use it personally.

To give majjit a try, just clone the repo and run `cargo run --release`.

When the program starts, you'll see the jujutsu log, as if you ran `jj log`. This can be navigated with hjkl and the arrow keys. You can also navigate with the mouse. Pressing tab will toggle folding/unfolding a node. If you hit tab on a commit, you will you will see the changed files, tab on a changed file will show the diff.

Various commands are already implemented:
- `f`: git fetch
- `p`: git pull
- `a`: abandon
- `s`: squash
- `c`: commit
- `e`: edit
- `d`: describe
- `n`: new
- `ctrl+r`: refresh the log tree
