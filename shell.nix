{ pkgs ? import <nixpkgs> { overlays = [ ]; } }:

let
  pkgsWithOverlay = import pkgs.path {
    system = pkgs.system;
    overlays = pkgs.overlays ++ [
      (import (builtins.fetchTarball {
        url = "https://github.com/oxalica/rust-overlay/archive/master.tar.gz";
      }))
    ];
  };
  rust = pkgsWithOverlay.rust-bin.stable.latest.default.override {
    targets = [ "x86_64-unknown-uefi" ];
  };
in
pkgsWithOverlay.mkShell {
  buildInputs = [
    rust
    pkgsWithOverlay.llvmPackages.libclang
  ];

  shellHook = ''
    export CARGO_TARGET_X86_64_UNKNOWN_UEFI_RUSTFLAGS="-C link-arg=-nostartfiles"
  '';
}
