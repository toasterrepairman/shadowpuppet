use adw::prelude::*;
use gtk4::{glib, gio};
use gtk4::{Application, ApplicationWindow, Picture, FileChooserNative, FileChooserAction, DrawingArea};
use std::cell::RefCell;
use std::rc::Rc;
use image::{DynamicImage, GenericImageView, RgbImage};
use std::fs::File;
use std::io::Write;
use adw::subclass::prelude::*;

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("com.example.Shadowpuppet")
        .build();

    app.connect_activate(build_ui);
    app.run()
}
fn build_ui(app: &Application) {
    let img_data: Rc<RefCell<Option<RgbImage>>> = Rc::new(RefCell::new(None));
    let num_layers = Rc::new(RefCell::new(8u8)); // default layers

    // Make the preview area larger and set a minimum size
    let preview_area = DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    preview_area.set_size_request(300, 300);

    // Improved drawing function
    {
        let img_data = img_data.clone();
        let num_layers = num_layers.clone();
        preview_area.set_draw_func(move |area, cr, width, height| {
            if let Some(ref img) = *img_data.borrow() {
                // Calculate scaling factors to fit the image in the preview area
                let scale_x = width as f64 / img.width() as f64;
                let scale_y = height as f64 / img.height() as f64;
                let scale = scale_x.min(scale_y);

                // Clear background
                cr.set_source_rgb(0.0, 0.0, 0.0);
                cr.paint().unwrap();

                // Apply scaling
                cr.scale(scale, scale);

                let scale_val = 255.0 / (*num_layers.borrow() as f32 - 1.0);

                for (x, y, pixel) in img.enumerate_pixels() {
                    let gray = (0.299 * pixel[0] as f32
                            + 0.587 * pixel[1] as f32
                            + 0.114 * pixel[2] as f32);
                    // Quantize the grayscale value
                    let quantized = ((gray / scale_val).round() * scale_val).clamp(0.0, 255.0);
                    let normalized = quantized / 255.0;

                    cr.set_source_rgb(normalized as f64, normalized as f64, normalized as f64);
                    cr.rectangle(x as f64, y as f64, 1.0, 1.0);
                    cr.fill().unwrap();
                }
            }
        });
    }

    // UI layout
    let open_button = gtk4::Button::from_icon_name("document-open-symbolic");
    let save_button = gtk4::Button::from_icon_name("document-save-symbolic");

    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("Depthmap App", ""))
        .build();

    header.pack_start(&open_button);
    header.pack_end(&save_button);

    // Improved slider configuration
    let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 2.0, 64.0, 1.0);
    slider.set_value(8.0);
    slider.set_draw_value(true);
    slider.set_margin_start(12);
    slider.set_margin_end(12);
    slider.set_margin_top(6);
    slider.set_margin_bottom(6);

    // Content box with proper spacing and margins
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(12)
        .build();

    content.append(&preview_area);
    content.append(&slider);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Depthmap App")
        .default_width(400)
        .default_height(500)
        .child(&content)
        .build();

    let file_chooser_ref = Rc::new(RefCell::new(None::<FileChooserNative>));

    // Open button handler
    {
        let app = app.clone();
        let img_data = img_data.clone();
        let preview_area = preview_area.clone();
        let file_chooser_ref = file_chooser_ref.clone();

        open_button.connect_clicked(move |_| {
            let file_chooser = FileChooserNative::builder()
                .title("Open Image")
                .action(FileChooserAction::Open)
                .accept_label("Open")
                .build();

            file_chooser.set_transient_for(Some(&app.active_window().unwrap()));

            // Store the file chooser
            *file_chooser_ref.borrow_mut() = Some(file_chooser.clone());

            file_chooser.connect_response({
                let img_data = img_data.clone();
                let preview_area = preview_area.clone();
                let file_chooser_ref = file_chooser_ref.clone();

                move |dialog, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = dialog.file() {
                            if let Some(path) = file.path() {
                                if let Ok(img) = image::open(path) {
                                    *img_data.borrow_mut() = Some(img.to_rgb8());
                                    preview_area.queue_draw();
                                }
                            }
                        }
                    }
                    dialog.destroy();
                    // Clear the reference after the dialog is destroyed
                    *file_chooser_ref.borrow_mut() = None;
                }
            });

            file_chooser.show();
        });
    }

    // Save button handler
    {
        let img_data = img_data.clone();
        let num_layers = num_layers.clone();
        save_button.connect_clicked(move |_| {
            if let Some(ref img) = *img_data.borrow() {
                save_as_obj(img, *num_layers.borrow(), "output.obj");
            }
        });
    }

    window.set_titlebar(Some(&header));
    window.present();
}

// Save grayscale .obj
fn save_as_obj(img: &RgbImage, layers: u8, path: &str) {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let scale = 0.1;
    let base_height = 0.0;
    let mut file = File::create(path).expect("Can't create OBJ file");

    let layer_scale = 1.0 / (layers as f32 - 1.0);

    writeln!(file, "mtllib material.mtl\nusemtl plane_material\nnewmtl plane_material\nKd 1.0 1.0 1.0\nKa 0.0 0.0 0.0").unwrap();

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x as u32, y as u32);
            let gray = 0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32;
            let quantized = (gray / 255.0 * (layers as f32 - 1.0)).round() * layer_scale;
            let z = quantized * scale + base_height;
            writeln!(file, "v {} {} {}", x as f32 * scale, y as f32 * scale, z).unwrap();
        }
    }

    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let v1 = y * width + x + 1;
            let v2 = y * width + x + 2;
            let v3 = (y + 1) * width + x + 2;
            let v4 = (y + 1) * width + x + 1;
            writeln!(file, "f {} {} {} {}", v1, v2, v3, v4).unwrap();
        }
    }
}
