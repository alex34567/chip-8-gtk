extern crate gtk;
extern crate chip_8_core;
extern crate sdl2;

mod sdl_sound;

use sdl_sound::SdlAudioWrapper;
use sdl_sound::SimpleAudioDevice;
use chip_8_core::*;
use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;

#[derive(Clone)]
struct GtkKeyWrapper(Rc<RefCell<u16>>);

impl GtkKeyWrapper {
    fn new() -> GtkKeyWrapper {
        GtkKeyWrapper(Rc::new(RefCell::new(0)))
    }
}

impl KeyWrapper for GtkKeyWrapper {
    fn is_pushed(&self, key: u8) -> Result<bool, &'static str> {
        let test = 1 << key;
        if test & *self.0.borrow() != 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn get_key(&self) -> Option<u8> {
        let keys = *self.0.borrow();
        for x in 0..15 {
            let key = 1 << x;
            if key & keys != 0 {
                return Some(x);
            }
        }
        None
    }
}

struct GtkChip8 {
    chip8: Chip8<GtkKeyWrapper, SdlAudioWrapper<SimpleAudioDevice>>,
    running: bool,
    paused: bool,
    rendered: bool,
    scale: f64,
}

impl GtkChip8 {
    fn new(key_wrap: GtkKeyWrapper) -> GtkChip8 {
        let audio_wrap = sdl_sound::init_sound();
        GtkChip8 {
            chip8: Chip8::new(key_wrap, audio_wrap),
            running: false,
            paused: false,
            rendered: false,
            scale: 0.0,
        }
    }
}

fn gen_error<T: IsA<gtk::Window> + WindowExt>(window: &T, error: &str) {
    let flags = gtk::DialogFlags::empty();
    let err = gtk::MessageDialog::new(
        Some(window), flags, gtk::MessageType::Error, gtk::ButtonsType::Ok,
        error);
    err.connect_response(move |err_window, _| {
        err_window.close();
    });
    err.set_modal(true);
    err.show_all();
}

fn open_file<T: IsA<gtk::Window> + IsA<gtk::FileChooser> + IsA<gtk::Object> + WindowExt>
(window: &T, gtk_chip8: &mut GtkChip8) {
    let wrapped = window.get_filename();
    if wrapped.is_none() {
        return;
    }
    let path = wrapped.unwrap();
    let wrapped = File::open(&path);
    if wrapped.is_err() {
        gen_error(window, "The file could not be opened");
        return
    }
    let mut file = wrapped.unwrap();
    let wrapped = file.metadata();
    if wrapped.is_err() {
        gen_error(window, "The file's metadata could be read");
        return
    }
    let metadata = wrapped.unwrap();
    if metadata.is_dir() {
        window.set_current_folder(&path);
        return
    }
    gtk_chip8.chip8.reboot();
    if gtk_chip8.chip8.load_prog_from_file(&mut file).is_err() {
        gen_error(window, "The file could not be read");
        return
    }
    gtk_chip8.running = true;
    window.close();
}

fn gdk_key_decode(key: u32) -> Option<u8> {
    match key {
        0x031 => Some(1),
        0x032 => Some(2),
        0x033 => Some(3),
        0x034 => Some(0xC),
        0x071 => Some(4),
        0x077 => Some(5),
        0x065 => Some(6),
        0x072 => Some(0xD),
        0x061 => Some(7),
        0x073 => Some(8),
        0x064 => Some(9),
        0x066 => Some(0xE),
        0x07A => Some(0xA),
        0x079 => Some(0),
        0x063 => Some(0xB),
        0x076 => Some(0xF),
        _ => None,
    }
}

fn main() {
    gtk::init().unwrap();
    let key_wrapper = GtkKeyWrapper::new();
    let key_wrapper_ref = key_wrapper.clone();
    let chip8 = Rc::new(RefCell::new(GtkChip8::new(key_wrapper_ref)));
    let builder = gtk::Builder::new_from_file("window.ui");
    let window: gtk::Window = builder.get_object("main_window").unwrap();
    let key_wrapper_ref = key_wrapper.clone();
    window.connect_key_press_event(move |_, key| {
        let mut keys = key_wrapper_ref.0.borrow_mut();
        if let Some(hex_key) = gdk_key_decode(key.get_keyval()) {
            let key = 1 << hex_key;
            *keys |= key;
        };
        Inhibit(false)
    });
    window.connect_key_release_event(move |_, key| {
        let mut keys = key_wrapper.0.borrow_mut();
        if let Some(hex_key) = gdk_key_decode(key.get_keyval()) {
            let key = 1 << hex_key;
            *keys ^= key;
        };
        Inhibit(false)
    });
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    window.show_all();
    let open: gtk::MenuItem = builder.get_object("open_menu").unwrap();
    let file_window: gtk::FileChooserDialog = builder.get_object("file_window").unwrap();
    let window_ref: gtk::FileChooserDialog = file_window.clone();
    let file_cancel: gtk::Button = builder.get_object("file_cancel").unwrap();
    let file_ok: gtk::Button = builder.get_object("file_ok").unwrap();
    file_cancel.connect_clicked(move |_| {
        window_ref.close();
    });
    let window_ref: gtk::FileChooserDialog = file_window.clone();
    let chip8_ref = chip8.clone();
    file_ok.connect_clicked(move |_| {
        open_file(&window_ref, &mut chip8_ref.borrow_mut())
    });
    let chip8_ref = chip8.clone();
    file_window.connect_delete_event(move |window, _| {
        let mut chip8_borrowed = chip8_ref.borrow_mut();
        chip8_borrowed.paused = false;
        window.hide();
        Inhibit(true)
    });
    let chip8_ref = chip8.clone();
    file_window.connect_file_activated(move |window| {
        open_file(window, &mut chip8_ref.borrow_mut())
    });
    let chip8_ref = chip8.clone();
    open.connect_activate(move |_| {
        let mut chip8_borrowed = chip8_ref.borrow_mut();
        chip8_borrowed.paused = true;
        file_window.show_all()
    });
    let quit: gtk::MenuItem = builder.get_object("quit_menu").unwrap();
    quit.connect_activate(|_| {
        gtk::main_quit()
    });
    let chip8_ref = chip8.clone();
    let draw_area: gtk::DrawingArea = builder.get_object("draw_area").unwrap();
    draw_area.connect_draw(move |_, context| {
        let mut chip8_borrowed = chip8_ref.borrow_mut();
        if !chip8_borrowed.rendered {
            let (_, _, width, hight) = context.clip_extents();
            chip8_borrowed.scale = width / 64.0;
            if hight / 32.0 < chip8_borrowed.scale {
                chip8_borrowed.scale = hight / 32.0;
            }
            chip8_borrowed.rendered = true;
        }
        let scale = chip8_borrowed.scale;
        context.set_source_rgb(0.0, 0.0, 0.0);
        context.paint();
        context.set_source_rgb(1.0, 1.0, 1.0);
        for (y_pos, y) in chip8_borrowed.chip8.frame_buffer.iter().enumerate() {
            for (x_pos, x) in y.iter().enumerate() {
                if *x == 1 {
                    context.rectangle(x_pos as f64 * scale, y_pos as f64 * scale, scale, scale)
                }
            }
        }
        context.fill();
        Inhibit(false)
    });
    gtk::timeout_add(16, move || {
        let mut chip8_borrowed = chip8.borrow_mut();
        let run = chip8_borrowed.running;
        let pause = chip8_borrowed.paused;
        if run & !pause {
            let err;
            if let Err(error) = chip8_borrowed.chip8.run_vblank() {
                gen_error(&window, error);
                err = true;
            } else {
                err = false;
            }
            if err {
                chip8_borrowed.running = false;
            }
        }
        chip8_borrowed.rendered = false;
        draw_area.queue_draw();
        Continue(true)
    });
    gtk::main();
}
