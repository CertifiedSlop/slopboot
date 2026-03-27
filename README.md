# slopboot

> **Minimalist, Memory-Safe UEFI Bootloader in Rust**

<div align="center">

[![CI](https://github.com/CertifiedSlop/slopboot/actions/workflows/ci.yml/badge.svg)](https://github.com/CertifiedSlop/slopboot/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://rust-lang.org)

![Status](https://img.shields.io/badge/Status-Experimental-orange)

</div>

---

## 🎯 What is slopboot?

`slopboot` is a blazing fast, ridiculously minimal, strict Boot Loader Specification (BLS) compliant UEFI bootloader written entirely in `no_std` Rust. 

There's no bloated configuration parsing and no pretty graphical menus. It just finds your Linux kernels on the EFI System Partition (ESP) and boots them instantly.

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🚀 **Instant Handoff** | Uses native UEFI `DevicePath` loading to bypass double-RAM allocation bottlenecks. |
| 🛡️ **Memory Safe** | Written in 100% pure Rust using `uefi-rs` with explicit, strict firmware pool allocations. |
| 📦 **BLS Compliant** | Fully supports Type 1 (Snippet `.conf`) and Type 2 (Unified Kernel Image `.efi`) boot entries. |
| ⌨️ **Event-Driven UI** | Zero CPU-spinning. Uses idiomatic UEFI `wait_for_event` timers for its 2-second interrupt loop. |
| 🔧 **Direct to Firmware** | Press `F` during boot to cleanly set `OSIndications` and reboot directly into your motherboard's UEFI setup UI. |
| 🧹 **Zero Bloat** | Compiles down to a tiny, standalone `.efi` executable. |

## 📦 Building from Source

### Prerequisites

- Rust toolchain (stable)
- `x86_64-unknown-uefi` target

### Build Instructions

```bash
# Clone the repository
git clone https://github.com/CertifiedSlop/slopboot.git
cd slopboot

# Add the UEFI target if you haven't already
rustup target add x86_64-unknown-uefi

# Build the EFI payload
cargo build --target x86_64-unknown-uefi --release
```

The compiled bootloader will be located at `target/x86_64-unknown-uefi/release/slopboot.efi`.

## 🐧 Building on NixOS

This project includes a Nix flake for reproducible builds on NixOS.

### Quick Start

One-off build without modifying your system configuration:

```bash
nix build github:CertifiedSlop/slopboot#slopboot
```

Or build from a local checkout:

```bash
git clone https://github.com/CertifiedSlop/slopboot.git
cd slopboot
nix build .#slopboot
```

### Prerequisites

Enable flakes if you haven't already. Add the following to your `/etc/nixos/configuration.nix`:

```nix
{
  nix.settings.experimental-features = [ "nix-command" "flakes" ];
}
```

Then rebuild your system:

```bash
sudo nixos-rebuild switch
```

### Adding slopboot as a Flake Input

To integrate slopboot into your NixOS configuration, add it as a flake input:

**`flake.nix`** (in your NixOS configuration directory, e.g., `/etc/nixos/flake.nix`):

```nix
{
  description = "My NixOS Configuration";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    slopboot.url = "github:CertifiedSlop/slopboot";
  };

  outputs = { self, nixpkgs, slopboot }: {
    nixosConfigurations.my-machine = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        # Optional: Use slopboot package from the flake
        ({ pkgs, ... }: {
          environment.systemPackages = [
            slopboot.packages.${pkgs.system}.slopboot
          ];
        })
      ];
    };
  };
}
```

### Referencing the Package in configuration.nix

Once added as a flake input, reference the slopboot package in your `configuration.nix`:

```nix
{ pkgs, ... }: {
  # Install slopboot to your system PATH for manual EFI installation
  environment.systemPackages = [
    pkgs.slopboot  # If using nixpkgs, or:
    # inputs.slopboot.packages.${pkgs.system}.slopboot  # If using flake input
  ];

  # Optional: Copy slopboot.efi to the EFI System Partition
  boot.loader.efi.efiSysMountPoint = "/boot";
  # Manual deployment after build:
  # sudo cp $(nix build .#slopboot --print-out-paths)/target/x86_64-unknown-uefi/release/slopboot.efi /boot/EFI/BOOT/BOOTX64.EFI
}
```

### Development Shell

Enter the development shell with all required dependencies:

```bash
nix develop
```

Inside the dev shell, you can build using standard Cargo commands:

```bash
cargo build --target x86_64-unknown-uefi --release
```

The dev shell provides:
- Rust toolchain (rustc, cargo)
- LLVM/clang for bindgen dependencies
- Pre-configured environment for UEFI target builds
- Custom `RUSTFLAGS` for UEFI target linking

**Run tests in the dev shell:**

```bash
nix develop
cargo test --target x86_64-unknown-uefi
```

**Format and lint:**

```bash
nix develop
cargo fmt --check
cargo clippy --target x86_64-unknown-uefi -- -D warnings
```

### Build Output Locations

| Build Method | Output Path | Access |
|--------------|-------------|--------|
| `nix build .#slopboot` | `./result/` | Symlink to `/nix/store/<hash>-slopboot-0.1.0/` |
| Flake package | `/nix/store/<hash>-slopboot-0.1.0/` | Read-only Nix store |
| Dev shell cargo build | `./target/x86_64-unknown-uefi/release/slopboot.efi` | Standard Cargo output |

**Access the built EFI binary:**

```bash
# After 'nix build .#slopboot':
ls -la result/target/x86_64-unknown-uefi/release/slopboot.efi

# Or print the exact store path:
nix build .#slopboot --print-out-paths
```

### Troubleshooting

| Issue | Solution |
|-------|----------|
| `error: experimental Nix feature 'flakes' is disabled` | Enable flakes in `/etc/nixos/configuration.nix` and run `sudo nixos-rebuild switch` |
| `libclang not found` or bindgen errors | Ensure `llvmPackages.libclang` is in `buildInputs` (already configured in flake) |
| Build fails with linker errors | Run `nix develop` first to get the correct `RUSTFLAGS` environment |
| `result` symlink not created | Use `nix build .#slopboot` (not `nix-build`) for flake builds |
| Cannot write to `/nix/store` | Nix store is immutable; use `nix develop` for local builds or copy from `result/` |
| QEMU test fails with OVMF errors | Ensure `ovmf` package is installed: `nix-shell -p qemu_kvm ovmf` |

**Get build logs:**

```bash
nix build .#slopboot -L
```

**Rebuild from scratch (clean build):**

```bash
nix build .#slopboot --rebuild
```

## 🧪 Testing Locally (QEMU)

We've decoupled the ESP into a volatile staging script, making local testing ridiculously easy.

1. Drop your favorite `vmlinuz`, `initramfs.img`, or UKI `.efi` kernels into the `test_binaries/` directory (it's automatically git-ignored!).
2. Run the automated QEMU runner:

```bash
./scripts/runqemu.sh
```

The script will:
- Recompile the bootloader automatically.
- Construct a volatile ESP structure in `build/esp/`.
- Deploy your test binaries and sample configurations into the virtual FAT tree.
- Launch QEMU utilizing KVM and OVMF firmware.

## 📖 Configuration

`slopboot` strictly follows the systemd-boot / Boot Loader Specification layout. 
Copy `slopboot.efi` to your ESP at `EFI/BOOT/BOOTX64.EFI`.

### Type 1: Drop-in Snippets
Place configuration files in `loader/entries/` on your ESP.

**Example `loader/entries/slop_os.conf`:**
```ini
title   SlopOS Minimal
linux   /EFI/Linux/vmlinuz
initrd  /EFI/Linux/initramfs.img
options root=PARTUUID=1234 quiet rw
```

### Type 2: Unified Kernel Images (UKI)
Place `.efi` UKI binaries directly into `EFI/Linux/` on the ESP. `slopboot` will automatically discover and present them in the boot menu.

## 📋 Boot Menu Controls

By default, `slopboot` will automatically boot the topmost discovered entry after a **2-second** non-blocking delay.

- `Spacebar`: Interrupt the auto-boot sequence and show the full selection menu.
- `Number Keys (1-9)`: Select and boot the corresponding entry.
- `F`: Set the `OsIndications` EFI variable and reset directly into the UEFI Firmware Setup interface.
- `Escape`: Resume the normal boot sequence.

## 🤝 Contributing

We welcome standard PRs. Because this interacts directly with motherboard firmwares, please ensure your changes pass our strict CI checks:

```bash
# Verify your code meets our pristine codebase standards
cargo fmt
cargo clippy --target x86_64-unknown-uefi -- -D warnings
```

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

