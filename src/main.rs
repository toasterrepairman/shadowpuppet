use adw::prelude::*;
use gtk4::{glib, gio};
use gtk4::{Application, ApplicationWindow, Picture, FileChooserNative, FileChooserAction, DrawingArea};
use std::cell::RefCell;
use std::rc::Rc;
use image::{DynamicImage, GenericImageView, RgbImage};
use std::fs::File;
use std::io::Write;
use adw::subclass::prelude::*;
use gio::SimpleAction;

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

    // Make the preview area
    let preview_area = DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    preview_area.set_size_request(300, 300);

    // Drawing function
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

    // Create the WindowTitle and store it in an Rc<RefCell> (for adjusting later)
    let window_title = adw::WindowTitle::new("Shadowpuppet", "");
    let window_title = Rc::new(RefCell::new(window_title));

    let header = adw::HeaderBar::builder()
        .title_widget(&*window_title.borrow())
        .css_classes(["flat"])
        .build();

    // Header config
    header.pack_start(&open_button);
    header.pack_end(&save_button);

    // Improved slider configuration
    let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 2.0, 64.0, 1.0);
    slider.set_value(8.0);
    slider.set_draw_value(true);
    slider.set_margin_start(0);
    slider.set_margin_end(18);
    slider.set_margin_top(0);
    slider.set_margin_bottom(12);
    slider.set_hexpand(true);
    slider.set_vexpand(false);

    // Create a SpinButton for numeric entry
    let spin_button = gtk4::SpinButton::with_range(2.0, 64.0, 1.0);
    spin_button.set_value(8.0);
    spin_button.set_digits(0);
    spin_button.set_width_chars(3);
    spin_button.set_vexpand(false);
    spin_button.set_valign(gtk4::Align::Center);

    // Create a horizontal box for slider and spin button
    let slider_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_top(0)
        .hexpand(true)
        .build();

    slider_box.append(&slider);
    slider_box.append(&spin_button);

    // Connect slider to spin button
    {
        let spin_button = spin_button.clone();
        slider.connect_value_changed(move |s| {
            spin_button.set_value(s.value());
        });
    }

    // Connect spin button to slider
    {
        let slider = slider.clone();
        let num_layers_for_spin = num_layers.clone();
        let preview_area_for_spin = preview_area.clone();
        spin_button.connect_value_changed(move |s| {
            slider.set_value(s.value());
            *num_layers_for_spin.borrow_mut() = s.value() as u8;
            preview_area_for_spin.queue_draw();
        });
    }

    // Slider value changed handler
    {
        let num_layers_for_slider = num_layers.clone();
        let preview_area_for_slider = preview_area.clone();
        slider.connect_value_changed(move |s| {
            *num_layers_for_slider.borrow_mut() = s.value() as u8;
            preview_area_for_slider.queue_draw();
        });
    }

    // Content box with proper spacing and margins
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_start(6)
        .margin_end(6)
        .margin_bottom(12)
        .build();

    content.append(&preview_area);
    content.append(&slider_box);

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
        let window_title = window_title.clone();  // Clone it here

        open_button.connect_clicked(move |_| {
            let file_chooser = FileChooserNative::builder()
                .title("Open Image")
                .action(FileChooserAction::Open)
                .accept_label("Open")
                .build();

            file_chooser.set_transient_for(Some(&app.active_window().unwrap()));

            *file_chooser_ref.borrow_mut() = Some(file_chooser.clone());

            file_chooser.connect_response({
                let img_data = img_data.clone();
                let preview_area = preview_area.clone();
                let file_chooser_ref = file_chooser_ref.clone();
                let window_title = window_title.clone();  // Clone it again for the closure

                move |dialog, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = dialog.file() {
                            if let Some(path) = file.path() {
                                // Update the subtitle with the filename
                                if let Some(filename) = path.file_name() {
                                    if let Some(filename_str) = filename.to_str() {
                                        window_title.borrow().set_subtitle(filename_str);
                                    }
                                }

                                if let Ok(img) = image::open(path) {
                                    *img_data.borrow_mut() = Some(img.to_rgb8());
                                    preview_area.queue_draw();
                                }
                            }
                        }
                    }
                    dialog.destroy();
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

    // ctrl + q close keybind
    let quit_action = SimpleAction::new("quit", None);
    quit_action.connect_activate(glib::clone!(@weak window => move |_, _| {
        window.close();
    }));
    window.add_action(&quit_action);

    app.set_accels_for_action("win.quit", &["<Control>q"]);
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
            // Negate the Y coordinate to rotate 180 degrees around X axis
            writeln!(file, "v {} {} {}",
                x as f32 * scale,
                -(y as f32 * scale),
                z
            ).unwrap();
        }
    }

    // When writing faces, we need to change the winding order to maintain correct face orientation
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let v1 = y * width + x + 1;
            let v2 = y * width + x + 2;
            let v3 = (y + 1) * width + x + 2;
            let v4 = (y + 1) * width + x + 1;
            // Reverse the order of vertices to maintain correct face orientation
            writeln!(file, "f {} {} {} {}", v1, v4, v3, v2).unwrap();
        }
    }
}
