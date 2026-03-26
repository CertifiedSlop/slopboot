#![no_main]
#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use uefi::boot::image_handle;
use uefi::prelude::*;
use uefi::proto::console::text::Key;
use uefi::proto::device_path::build::{media::FilePath, DevicePathBuilder};
use uefi::proto::device_path::DevicePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, FileType};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{cstr16, CString16};

#[derive(Debug, Clone)]
enum BootEntry {
    Type1 {
        linux: String,
        initrd: Option<String>,
        options: String,
        title: String,
    },
    Type2 {
        title: String,
        path: String,
    },
}

impl BootEntry {
    fn title(&self) -> &str {
        match self {
            BootEntry::Type1 { title, .. } => title,
            BootEntry::Type2 { title, .. } => title,
        }
    }
}

fn fallback(msg: &str) -> Status {
    uefi::println!("ERROR: {}", msg);
    uefi::boot::stall(core::time::Duration::from_secs(5));
    Status::LOAD_ERROR
}

fn read_file(root: &mut Directory, path: &str) -> Option<Vec<u8>> {
    let path_16 = CString16::try_from(path).ok()?;
    let mut file = root
        .open(&path_16, FileMode::Read, FileAttribute::empty())
        .ok()?
        .into_regular_file()?;

    let mut info_buf = vec![0; 512];
    let info = file.get_info::<FileInfo>(&mut info_buf).ok()?;
    let size = info.file_size() as usize;

    let mut data = vec![0; size];
    file.read(&mut data).ok()?;
    Some(data)
}

fn scan_type2(root: &mut Directory) -> Vec<BootEntry> {
    let mut entries = Vec::new();
    let path = cstr16!("EFI\\Linux");

    let mut dir = match root.open(path, FileMode::Read, FileAttribute::empty()) {
        Ok(f) => match f.into_type() {
            Ok(FileType::Dir(d)) => d,
            _ => return entries,
        },
        Err(_) => return entries,
    };

    let mut buf = vec![0; 512];
    while let Ok(Some(info)) = dir.read_entry(&mut buf) {
        let name = info.file_name();
        let name_str = name.to_string();
        if name_str.ends_with(".efi") {
            entries.push(BootEntry::Type2 {
                title: name_str.clone(),
                path: alloc::format!("EFI\\Linux\\{}", name_str),
            });
        }
    }
    entries
}

fn split_kv(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    let idx = line.find(|c: char| c.is_ascii_whitespace())?;
    Some((line[..idx].trim(), line[idx..].trim()))
}

fn parse_type1(id: &str, content: &str) -> Option<BootEntry> {
    let mut title = String::new();
    let mut linux = String::new();
    let mut initrd = None;
    let mut options = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some((k, v)) = split_kv(line) {
            match k {
                "title" => title = v.into(),
                "linux" => linux = v.replace("/", "\\"),
                "initrd" => initrd = Some(v.replace("/", "\\")),
                "options" => options = v.into(),
                _ => {}
            }
        }
    }

    if linux.is_empty() {
        return None;
    }
    if title.is_empty() {
        title = id.into();
    }

    Some(BootEntry::Type1 {
        linux,
        initrd,
        options,
        title,
    })
}

fn scan_type1(root: &mut Directory) -> Vec<BootEntry> {
    let mut entries = Vec::new();
    let path = cstr16!("loader\\entries");

    let mut dir = match root.open(path, FileMode::Read, FileAttribute::empty()) {
        Ok(f) => match f.into_type() {
            Ok(FileType::Dir(d)) => d,
            _ => return entries,
        },
        Err(_) => return entries,
    };

    let mut buf = vec![0; 512];
    while let Ok(Some(info)) = dir.read_entry(&mut buf) {
        let name = info.file_name();
        let name_str = name.to_string();
        if name_str.ends_with(".conf") {
            let file_path = alloc::format!("loader\\entries\\{}", name_str);
            if let Some(content_bytes) = read_file(root, &file_path) {
                if let Ok(text) = String::from_utf8(content_bytes) {
                    if let Some(entry) = parse_type1(&name_str, &text) {
                        entries.push(entry);
                    }
                }
            }
        }
    }
    entries
}

fn boot_entry(entry: &BootEntry, device_handle: Handle) -> Status {
    let (kernel_path, options) = match entry {
        BootEntry::Type2 { path, .. } => (path.clone(), String::new()),
        BootEntry::Type1 {
            linux,
            initrd,
            options,
            ..
        } => {
            let mut opts = options.clone();
            if let Some(ird) = initrd {
                opts.push_str(" initrd=");
                opts.push_str(ird);
            }
            (linux.clone(), opts)
        }
    };

    let kernel_path_clean = if let Some(stripped) = kernel_path.strip_prefix('\\') {
        stripped
    } else {
        kernel_path.as_str()
    };

    let kernel_path_16 = match CString16::try_from(kernel_path_clean) {
        Ok(p) => p,
        Err(_) => return fallback("Invalid kernel path string format"),
    };

    // 1. Get the base device path of the partition we are booting from
    let base_device_path = match uefi::boot::open_protocol_exclusive::<DevicePath>(device_handle) {
        Ok(dp) => dp,
        Err(_) => return fallback("Failed to get base device path protocol"),
    };

    // 2. Build a standalone DevicePath for the file path
    let mut file_path_vec = alloc::vec::Vec::new();
    let builder = DevicePathBuilder::with_vec(&mut file_path_vec);
    let file_path_node = FilePath {
        path_name: &kernel_path_16,
    };

    let file_device_path = match builder.push(&file_path_node) {
        Ok(b) => match b.finalize() {
            Ok(dp) => dp,
            Err(_) => return fallback("Failed to finalize file device path"),
        },
        Err(_) => return fallback("Failed to push file path node"),
    };

    // 3. Append the file path to the base device path to form the full resolution target
    let full_device_path_pool = match base_device_path.append_path(file_device_path) {
        Ok(dp) => dp,
        Err(_) => return fallback("Failed to append file path to device handle"),
    };

    // 4. Let UEFI handle the loading natively from the constructed path
    let source = uefi::boot::LoadImageSource::FromDevicePath {
        device_path: &full_device_path_pool,
        boot_policy: uefi::proto::BootPolicy::ExactMatch,
    };

    let loaded_handle = match uefi::boot::load_image(image_handle(), source) {
        Ok(h) => h,
        Err(e) => return fallback(&alloc::format!("LoadImage failed: {:?}", e.status())),
    };

    // 4. Pass the command line options to the loaded kernel
    let load_options_ptr = if !options.is_empty() {
        if let Ok(mut loaded_image) =
            uefi::boot::open_protocol_exclusive::<LoadedImage>(loaded_handle)
        {
            let mut opts_16: Vec<u16> = options.encode_utf16().collect();
            opts_16.push(0); // Null terminate
            let size = opts_16.len() * 2;

            if let Ok(ptr) = uefi::boot::allocate_pool(uefi::boot::MemoryType::LOADER_DATA, size) {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        opts_16.as_ptr() as *const u8,
                        ptr.as_ptr(),
                        size,
                    );
                    loaded_image.set_load_options(ptr.as_ptr() as *const u8, size as u32);
                }
                Some(ptr)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 5. Hand over control to the kernel
    let res = uefi::boot::start_image(loaded_handle);

    if let Some(ptr) = load_options_ptr {
        unsafe {
            let _ = uefi::boot::free_pool(ptr);
        }
    }

    if let Err(e) = res {
        return fallback(&alloc::format!("StartImage failed: {:?}", e.status()));
    }

    Status::SUCCESS
}

#[entry]
fn main() -> Status {
    match uefi::helpers::init() {
        Ok(_) => {}
        Err(_) => return Status::LOAD_ERROR,
    };

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.clear();
    });

    let handle = image_handle();
    let loaded_image = match uefi::boot::open_protocol_exclusive::<LoadedImage>(handle) {
        Ok(lip) => lip,
        Err(_) => return fallback("Failed to open LoadedImage protocol"),
    };

    let device_handle = match loaded_image.device() {
        Some(h) => h,
        None => return fallback("No device handle in LoadedImage"),
    };

    let mut sfs = match uefi::boot::open_protocol_exclusive::<SimpleFileSystem>(device_handle) {
        Ok(sfs) => sfs,
        Err(_) => return fallback("Failed to open SimpleFileSystem"),
    };

    let mut root_dir = match sfs.open_volume() {
        Ok(dir) => dir,
        Err(_) => return fallback("Failed to open ESP root"),
    };

    let mut entries = scan_type2(&mut root_dir);
    entries.append(&mut scan_type1(&mut root_dir));

    if entries.is_empty() {
        return fallback("No boot entries found");
    }

    entries.sort_by(|a, b| b.title().cmp(a.title()));

    uefi::println!("slopboot");
    uefi::println!("Booting {} in 2 seconds.", entries[0].title());
    uefi::println!("Press Space to interrupt.");

    let mut interrupted = false;

    let timer = unsafe {
        uefi::boot::create_event(
            uefi::boot::EventType::TIMER,
            uefi::boot::Tpl::APPLICATION,
            None,
            None,
        )
        .unwrap()
    };
    uefi::boot::set_timer(&timer, uefi::boot::TimerTrigger::Relative(20_000_000)).unwrap();

    let stdin_event = uefi::system::with_stdin(|stdin| unsafe {
        stdin.wait_for_key_event().unwrap().unsafe_clone()
    });

    loop {
        let mut events = [unsafe { stdin_event.unsafe_clone() }, unsafe {
            timer.unsafe_clone()
        }];

        let idx = uefi::boot::wait_for_event(&mut events).unwrap_or(1);

        if idx == 1 {
            break; // timer fired
        }

        let mut pressed = None;
        uefi::system::with_stdin(|stdin| {
            if let Ok(Some(key)) = stdin.read_key() {
                pressed = Some(key);
            }
        });

        if let Some(Key::Printable(c)) = pressed {
            let ch = core::char::from_u32(u16::from(c) as u32).unwrap_or(' ');
            if ch == ' ' {
                interrupted = true;
                break;
            }
        }
    }

    let _ = uefi::boot::close_event(timer);

    let mut selected = 0;

    if interrupted {
        let _ = uefi::system::with_stdout(|stdout| stdout.clear());
        uefi::println!("slopboot Boot Options");
        uefi::println!("---------------------");
        for (i, e) in entries.iter().enumerate() {
            uefi::println!("{}. {}", i + 1, e.title());
        }
        uefi::println!("");
        uefi::println!("Press corresponding number to boot");
        uefi::println!("Press F to exit to firmware setup");
        uefi::println!("Press Esc to continue normal boot");

        loop {
            let mut pressed = None;
            uefi::system::with_stdin(|stdin| {
                if let Ok(Some(key)) = stdin.read_key() {
                    pressed = Some(key);
                }
            });

            match pressed {
                Some(Key::Printable(c)) => {
                    let ch = core::char::from_u32(u16::from(c) as u32).unwrap_or(' ');
                    if ch == 'f' || ch == 'F' {
                        let name = cstr16!("OsIndications");
                        let vendor = uefi::runtime::VariableVendor::GLOBAL_VARIABLE;
                        // EFI_OS_INDICATIONS_BOOT_TO_FW_UI (bit 0)
                        let indications: u64 = 1;
                        let attrs = uefi::runtime::VariableAttributes::NON_VOLATILE
                            | uefi::runtime::VariableAttributes::BOOTSERVICE_ACCESS
                            | uefi::runtime::VariableAttributes::RUNTIME_ACCESS;
                        let _ = uefi::runtime::set_variable(
                            name,
                            &vendor,
                            attrs,
                            &indications.to_le_bytes(),
                        );
                        uefi::runtime::reset(uefi::runtime::ResetType::COLD, Status::SUCCESS, None);
                    }
                    if let Some(digit) = ch.to_digit(10) {
                        let idx = (digit as usize).saturating_sub(1);
                        if idx < entries.len() {
                            selected = idx;
                            break;
                        }
                    }
                }
                Some(Key::Special(uefi::proto::console::text::ScanCode::ESCAPE)) => {
                    selected = 0;
                    break;
                }
                _ => {}
            }
            uefi::boot::stall(core::time::Duration::from_millis(10));
        }
    }

    let _ = uefi::system::with_stdout(|stdout| stdout.clear());
    let entry = &entries[selected];
    uefi::println!("Booting: {}", entry.title());

    boot_entry(entry, device_handle)
}
