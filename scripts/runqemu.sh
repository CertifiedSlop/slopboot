#!/usr/bin/env bash
set -e

# Change to project root relative to script
cd "$(dirname "$0")/.."

echo "[=================> Building <=================]"
cargo build --target x86_64-unknown-uefi --release
echo ""

echo "[=================> Assembling volatile ESP <=================]"
rm -rf build/esp
mkdir -p build/esp/EFI/BOOT
mkdir -p build/esp/EFI/Linux
mkdir -p build/esp/loader/entries

# EFI Standard target bootloader path
cp target/x86_64-unknown-uefi/release/slopboot.efi build/esp/EFI/BOOT/BOOTX64.EFI

# Custom configuration file
cp assets/test_loader_entry.conf build/esp/loader/entries/slop_os.conf

if ls test_binaries/*.efi 1> /dev/null 2>&1; then
    echo "Found UKI (.efi) images, copying to ESP Native Loader..."
    cp test_binaries/*.efi build/esp/EFI/Linux/
fi

if [ ! -f "test_binaries/vmlinuz" ] && ! ls test_binaries/*.efi 1> /dev/null 2>&1; then
    echo "WARNING: Kernel not found at 'test_binaries/vmlinuz' and no UKI (.efi) kernels found in 'test_binaries'."
    echo "If you want to test actually booting Linux, please download a kernel and place it there."
fi

if [ -f "test_binaries/vmlinuz" ]; then
    echo "Found custom vmlinuz image, mapping to ESP Native Loader..."
    cp test_binaries/vmlinuz build/esp/EFI/Linux/vmlinuz
fi

if [ -f "test_binaries/initramfs.img" ]; then
    echo "Found initramfs..."
    cp test_binaries/initramfs.img build/esp/EFI/Linux/initramfs.img
fi
echo ""

echo "[=================> Starting QEMU <=================]"
echo "(You may need to change the OVMF firmware path below if it errors out with a not found error)"
qemu-system-x86_64 \
    -enable-kvm \
    -m 1G \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/x64/OVMF_CODE.4m.fd \
    -drive format=raw,file=fat:rw:build/esp \
    -serial stdio \
    -net none