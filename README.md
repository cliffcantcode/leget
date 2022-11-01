# leget
A CLI tool to quickly explore data about LEGO sets.

## Build from source
Right now there are no precompiled binaries so you'll have to:
1. download the code from here
2. have the rust programming language installed
3. compile the code to a binary
4. move that binary to your PATH and allow it to be executed

### Here we go
1. download the code from here
there should be a button near the top of the page that will give you a link to this repo.
copy that and paste it into a terminal. then run git clone with that link.
something like this:
git clone https://github.com/cliffcantcode/leget.git

2. have the rust programming language installed
from here https://www.rust-lang.org/
you should be able to just follow the instructions for the lastest version. 

3. compile the code to a binary
in the terminal move into the leget directory and run:
cargo build --release
this should create a new directory called "target". inside of which
will should be something like release/leget depending on your operating system.
that "leget" is your binary.

4. move that binary to your PATH and allow it to be executed
Here is the walkthrough I used:
https://zwbetz.com/how-to-add-a-binary-to-your-path-on-macos-linux-windows/
it's a much better explanation than I could give.
