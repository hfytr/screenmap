This repo is organized into 2 rust projects:
- screenmap/process_csv is used to read a csv into the pgsql database. It can be run with:
    $ cargo run -- /path/to/csv/file
- screenmap/screenmap is the actual leptos website.

screenmap runs a version of leptos which requires rust nightly. Im not sure
which all versions of rust will work, but a sure fire way is to use the nix
flake. Look here for information about nix flakes: https://nixos.wiki/wiki/flakes.
If you want to chance other rust versions, go ahead. Look here for how to
install rust the "normal" way: https://www.rust-lang.org/tools/install

I run the website with flakes using the following commands:
screenmap > nix develop # to enter the rust development environment
screenmap > pg_ctl -D $PGDATA -l $PGLOG start # to start the postgres server
screenmap > cd screenmap
screenmap/screenmap > cargo-leptos watch # to begin serving the website

Once you are done, hit <C-c>, then run the following commands:
screenmap/screenmap > cd ..
screenmap > pg_ctl -D $PGDATA -l $PGLOG stop # to stop the postgres server
