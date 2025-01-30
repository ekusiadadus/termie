use eframe::egui;
use std::{
    ffi::{CStr, CString},
    fs::File,
    io::Read,
    os::fd::{AsRawFd, OwnedFd},
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
                let shell_name = CStr::from_bytes_with_nul(b"dash\0").unwrap();
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
    buf: Vec<u8>,
    fd: OwnedFd,
}

impl TermieGui {
    fn new(cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        let flags = nix::fcntl::fcntl(fd.as_raw_fd(), nix::fcntl::FcntlArg::F_GETFL).unwrap();
        let mut flags = nix::fcntl::OFlag::from_bits_truncate(flags);
        flags.set(nix::fcntl::OFlag::O_NONBLOCK, true);
        nix::fcntl::fcntl(fd.as_raw_fd(), nix::fcntl::FcntlArg::F_SETFL(flags)).unwrap();
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        TermieGui {
            buf: Vec::new(),
            fd,
        }
    }
}

impl eframe::App for TermieGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut buf = vec![0u8; 1024];
        match unsafe { nix::unistd::read(self.fd.as_raw_fd(), &mut buf) } {
            Ok(read_size) => {
                self.buf.extend_from_slice(&buf[..read_size]);
            }
            Err(e) => {
                if e != nix::errno::Errno::EAGAIN {
                    println!("Error: {:?}", e);
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| unsafe {
            ui.label(std::str::from_utf8_unchecked(&self.buf));
        });

        ctx.request_repaint();
    }
}
