{
  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url  = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustpkg = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rustfmt" "rustc-dev" "llvm-tools-preview" ];
          targets = [ "arm-unknown-linux-gnueabihf" "wasm32-unknown-unknown" ];
        });
        postgresqlConf = pkgs.writeText "postgresql.conf" ''
          listen_addresses = 'localhost'
          unix_socket_directories = '/home/fbwdw/docs/mgh/code/webapp/pgdata/run'
          port = 5432
          max_connections = 100
        '';
      in {
        devShells.default = with pkgs; mkShell rec {
          nativeBuildInputs = [
            pkg-config
            rustpkg
            cargo-leptos
            cargo-generate
            leptosfmt
            sass
            postgresql
            binaryen
          ];
          LD_LIBRARY_PATH = "${lib.makeLibraryPath nativeBuildInputs}";
          RUST_BACKTRACE = 1;
          PGDATA = "./pgdata";
          TESTPG = "${self.outPath}";
          PGLOG = "./pgdata/pglog.txt";
          DATABASE_URL = "postgres://fbwdw:password@127.0.0.1:5432/postgres";
          RA_LOG="rust_analyzer=info";
          shellHook = ''
            mkdir -p pgdata
            if [ ! -f "pgdata/PG_VERSION" ]; then
              ${pkgs.postgresql}/bin/initdb -D pgdata
            fi
            if [ ! -d "pgdata/run" ]; then
              mkdir pgdata/run
            fi
            cp ${postgresqlConf} pgdata/postgresql.conf
          '';
        };
      }
    );
}
