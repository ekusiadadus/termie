use eframe::egui;
use std::{
    ffi::{CStr, CString},
    fs::File,
    io::Read,
    os::fd::OwnedFd,
};

fn main() {
    let fd = unsafe {
        let res = nix::pty::forkpty(None, None).unwrap();
        match res {
            nix::pty::ForkptyResult::Parent { child, master } => {
                println!("Parent: {:?}", child);
                master
            }
            nix::pty::ForkptyResult::Child => {
                let shell_name = CStr::from_bytes_with_nul(b"ash\0").unwrap();
                nix::unistd::execvp::<CString>(&shell_name, &[]).expect("Failed to exec");
                unreachable!();
            }
        }
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(move |cc| Ok(Box::new(TermieGui::new(cc, fd)))),
    );
}

struct TermieGui {
    buf: String,
    fd: File,
}

impl TermieGui {
    fn new(cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        TermieGui {
            buf: String::new(),
            fd: unsafe { File::from(fd) },
        }
    }
}

impl eframe::App for TermieGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut buf = vec![0u8; 1024];
        match self.fd.read(&mut buf) {
            Ok(read_size) => {
                if read_size > 0 {
                    if let Ok(text) = String::from_utf8(buf[..read_size].to_vec()) {
                        self.buf.push_str(&text);
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(&self.buf);
        });
    }
}
