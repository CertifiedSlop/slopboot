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

