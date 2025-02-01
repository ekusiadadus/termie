use eframe::egui;
use std::{
    ffi::{CStr, CString},
    os::fd::{AsRawFd, OwnedFd},
};

fn get_character_size(ctx: &egui::Context) -> (f32, f32) {
    let font_id = ctx.style().text_styles[&egui::TextStyle::Monospace].clone();
    ctx.fonts(|fonts| {
        let layout = fonts.layout(
            "@".to_string(),
            font_id,
            egui::Color32::default(),
            f32::INFINITY,
        );
        (layout.rect.width(), layout.rect.height())
    })
}

fn character_to_cursor_offset(
    character_size: (f32, f32),
    cursor_pos: (usize, usize),
) -> (f32, f32) {
    (
        character_size.0 * cursor_pos.1 as f32,
        character_size.1 * cursor_pos.0 as f32,
    )
}

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
    let _ = eframe::run_native(
        "Termie",
        native_options,
        Box::new(move |cc| Ok(Box::new(TermieGui::new(cc, fd)))),
    );
}

struct TermieGui {
    buf: Vec<u8>,
    cursor_pos: (usize, usize), // (行, 列)
    character_size: Option<(f32, f32)>,
    fd: OwnedFd,
}

impl TermieGui {
    fn new(cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        cc.egui_ctx.style_mut(|style| {
            style.override_text_style = Some(egui::TextStyle::Monospace);
        });
        // 非同期読み出しの設定
        let flags = nix::fcntl::fcntl(fd.as_raw_fd(), nix::fcntl::FcntlArg::F_GETFL).unwrap();
        let mut flags =
            nix::fcntl::OFlag::from_bits_truncate(flags & nix::fcntl::OFlag::O_ACCMODE.bits());
        flags.set(nix::fcntl::OFlag::O_NONBLOCK, true);
        nix::fcntl::fcntl(fd.as_raw_fd(), nix::fcntl::FcntlArg::F_SETFL(flags)).unwrap();

        TermieGui {
            buf: Vec::new(),
            cursor_pos: (0, 0),
            character_size: None,
            fd,
        }
    }
}

impl eframe::App for TermieGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.character_size.is_none() {
            self.character_size = Some(get_character_size(ctx));
        }
        let mut read_buf = [0u8; 4096];
        match nix::unistd::read(self.fd.as_raw_fd(), &mut read_buf) {
            Ok(read_size) => {
                let incoming = &read_buf[..read_size];
                for &c in incoming {
                    match c {
                        // Backspace: 0x08 または 0x7f
                        0x08 | 0x7f => {
                            if self.cursor_pos.1 > 0 {
                                self.buf.pop();
                                self.cursor_pos.1 -= 1;
                            }
                        }
                        // 改行
                        0x0a => {
                            self.buf.push(c);
                            self.cursor_pos.0 += 1;
                            self.cursor_pos.1 = 0;
                        }
                        _ => {
                            self.buf.push(c);
                            self.cursor_pos.1 += 1;
                        }
                    }
                }
            }
            Err(e) => {
                if e != nix::errno::Errno::EAGAIN {
                    println!("Error: {:?}", e);
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.input(|input_state| {
                for event in &input_state.events {
                    let text = match event {
                        egui::Event::Text(text) => {
                            println!("Text input: {:?}", text);
                            text
                        }
                        egui::Event::Copy | egui::Event::Cut => continue,
                        egui::Event::Key {
                            key, pressed: true, ..
                        } => {
                            println!("Key pressed: {:?}", key);
                            match key {
                                egui::Key::Enter => "\n",
                                egui::Key::Tab => "\t",
                                egui::Key::Backspace => "\x7f",
                                egui::Key::Delete => "\x1b[3~",
                                egui::Key::Escape => "\x1b",
                                egui::Key::Insert => "\x1b[2~",
                                egui::Key::Home => "\x1b[1~",
                                egui::Key::End => "\x1b[4~",
                                egui::Key::PageUp => "\x1b[5~",
                                egui::Key::PageDown => "\x1b[6~",
                                egui::Key::ArrowUp => "\x1b[A",
                                egui::Key::ArrowDown => "\x1b[B",
                                egui::Key::ArrowRight => "\x1b[C",
                                egui::Key::ArrowLeft => "\x1b[D",
                                _ => continue,
                            }
                        }
                        _ => continue,
                    };

                    let bytes = text.as_bytes();
                    println!("Sending bytes: {:?}", bytes);
                    let mut to_write = bytes;
                    while !to_write.is_empty() {
                        // 修正: self.fd.as_raw_fd() -> &self.fd
                        let written = nix::unistd::write(&self.fd, to_write).unwrap();
                        to_write = &to_write[written..];
                    }
                }
            });

            let text = std::str::from_utf8(&self.buf).unwrap_or("<Invalid UTF-8>");
            let response = ui.label(text);
            let top_left = response.rect.min;
            let painter = ui.painter();
            let character_size = self.character_size.unwrap();
            let cursor_offset = character_to_cursor_offset(character_size, self.cursor_pos);
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::Pos2::new(top_left.x + cursor_offset.0, top_left.y + cursor_offset.1),
                    egui::Vec2::new(character_size.0, character_size.1),
                ),
                0.0,
                egui::Color32::GRAY,
            );
        });

        ctx.request_repaint();
    }
}
