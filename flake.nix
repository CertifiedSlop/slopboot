{
  description = "Minimalist, Memory-Safe UEFI Bootloader in Rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rust = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "x86_64-unknown-uefi" ];
        };
        rustPlatform = pkgs.rustPlatform;
      in
      {
        packages.slopboot = rustPlatform.buildRustPackage {
          pname = "slopboot";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          buildInputs = [ pkgs.llvmPackages.libclang ];

          nativeBuildInputs = [ pkgs.llvmPackages.libclang ];

          meta = with pkgs.lib; {
            description = "Minimalist, Memory-Safe UEFI Bootloader in Rust";
            homepage = "https://github.com/CertifiedSlop/slopboot";
            license = licenses.mit;
            maintainers = [ ];
          };
        };

        defaultPackage = self.packages.${system}.slopboot;

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rust
            pkgs.llvmPackages.libclang
          ];

          shellHook = ''
            export CARGO_TARGET_X86_64_UNKNOWN_UEFI_RUSTFLAGS="-C link-arg=-nostartfiles"
          '';
        };
      });
}
