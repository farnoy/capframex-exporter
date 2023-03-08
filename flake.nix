{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/22.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }@attrs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
          stdenv = pkgs.stdenv;
        in
        {
          packages = flake-utils.lib.flattenTree {
            capframex-exporter =
              pkgs.rustPlatform.buildRustPackage rec {
                pname = "capframex-exporter";
                version = "0.1.0";

                src = ./.;

                cargoSha256 = "sha256-rkINAbyzqvUfqBhb4ehQUnOyHUMM3x1pAuUnUWBKAgU=";
                doCheck = false;

                buildInputs = [ pkgs.openssl ];
                nativeBuildInputs = [ pkgs.pkg-config ];
              };


          };
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              cmake
              rustfmt
              pkg-config
              llvmPackages_14.bintools
              openssl
              cargo
            ];

            # WTF https://gist.github.com/hawkw/95375a1dc3cb1e740c323f25a00476ce#file-shell-nix-L22
            shellHook = with { inherit (pkgs) lib; }; ''
              export LIBCLANG_PATH="${pkgs.llvmPackages_14.libclang.lib}/lib"
              # export LD_LIBRARY_PATH="${stdenv.cc.cc.lib}/lib"

              export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld"

              export BINDGEN_EXTRA_CLANG_ARGS="$(< ${stdenv.cc}/nix-support/libc-crt1-cflags) \
                $(< ${stdenv.cc}/nix-support/libc-cflags) \
                $(< ${stdenv.cc}/nix-support/cc-cflags) \
                $(< ${stdenv.cc}/nix-support/libcxx-cxxflags) \
                ${
                  lib.optionalString stdenv.cc.isClang
                  "-idirafter ${stdenv.cc.cc}/lib/clang/${
                    lib.getVersion stdenv.cc.cc
                  }/include"
                } \
                ${
                  lib.optionalString stdenv.cc.isGNU
                  "-isystem ${stdenv.cc.cc}/include/c++/${
                    lib.getVersion stdenv.cc.cc
                  } -isystem ${stdenv.cc.cc}/include/c++/${
                    lib.getVersion stdenv.cc.cc
                  }/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${
                    lib.getVersion stdenv.cc.cc

                  }/include"
                } \
                "
            '';
          };
          formatter = pkgs.nixpkgs-fmt;
        }
      );
}
