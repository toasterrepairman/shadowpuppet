use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::SimpleAction;
use gtk4::cairo;
use gtk4::{gio, glib};
use gtk4::{Application, DrawingArea, FileChooserAction};
use image::{DynamicImage, GenericImageView, RgbImage};
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("com.example.Shadowpuppet")
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let img_data: Rc<RefCell<Option<RgbImage>>> = Rc::new(RefCell::new(None));
    let num_layers = Rc::new(RefCell::new(8u8)); // default layers

    // Create toast overlay for notifications
    let toast_overlay = adw::ToastOverlay::new();

    // Make the preview area
    let preview_area = DrawingArea::builder().hexpand(true).vexpand(true).build();

    preview_area.set_size_request(300, 300);

    // Cache for the processed image surface
    let cached_surface: Rc<RefCell<Option<cairo::ImageSurface>>> = Rc::new(RefCell::new(None));
    let cached_layers: Rc<RefCell<u8>> = Rc::new(RefCell::new(8));

    // Drawing function
    {
        let img_data = img_data.clone();
        let num_layers = num_layers.clone();
        let cached_surface = cached_surface.clone();
        let cached_layers = cached_layers.clone();

        preview_area.set_draw_func(move |area, cr, width, height| {
            let theme_bg = area.style_context().lookup_color("window_bg_color");
            if let Some(color) = theme_bg {
                cr.set_source_rgba(
                    color.red() as f64,
                    color.green() as f64,
                    color.blue() as f64,
                    color.alpha() as f64,
                );
            } else {
                cr.set_source_rgb(0.15, 0.15, 0.15);
            }
            cr.paint().unwrap();

            if let Some(ref img) = *img_data.borrow() {
                let current_layers = *num_layers.borrow();

                // Regenerate surface if layers changed or surface doesn't exist
                if cached_surface.borrow().is_none() || *cached_layers.borrow() != current_layers {
                    let img_width = img.width() as i32;
                    let img_height = img.height() as i32;

                    // Create an image surface for the processed image
                    let mut surface =
                        cairo::ImageSurface::create(cairo::Format::Rgb24, img_width, img_height)
                            .unwrap();

                    {
                        let scale_val = 255.0 / (current_layers as f32 - 1.0);
                        let stride = surface.stride() as usize;
                        let width = img_width as usize;
                        let height = img_height as usize;

                        let mut data = surface.data().unwrap();
                        let pixels = img.as_raw();

                        for y in 0..height {
                            let row_offset = y * stride;
                            let src_offset = y * width * 3;
                            for x in 0..width {
                                let si = src_offset + x * 3;
                                let r = pixels[si] as f32;
                                let g = pixels[si + 1] as f32;
                                let b = pixels[si + 2] as f32;
                                let gray = 0.299 * r + 0.587 * g + 0.114 * b;
                                let quantized =
                                    ((gray / scale_val).round() * scale_val).clamp(0.0, 255.0);
                                let v = quantized as u8;
                                let di = row_offset + x * 4;
                                data[di] = v;
                                data[di + 1] = v;
                                data[di + 2] = v;
                            }
                        }
                    }

                    *cached_surface.borrow_mut() = Some(surface);
                    *cached_layers.borrow_mut() = current_layers;
                }

                // Draw the cached surface with proper scaling and centering
                if let Some(ref surface) = *cached_surface.borrow() {
                    let img_width = img.width() as f64;
                    let img_height = img.height() as f64;

                    // Calculate scaling to fit within the drawing area
                    let scale_x = width as f64 / img_width;
                    let scale_y = height as f64 / img_height;
                    let scale = scale_x.min(scale_y);

                    // Calculate centering offset
                    let scaled_width = img_width * scale;
                    let scaled_height = img_height * scale;
                    let offset_x = (width as f64 - scaled_width) / 2.0;
                    let offset_y = (height as f64 - scaled_height) / 2.0;

                    // Apply transformations
                    cr.save().unwrap();
                    cr.translate(offset_x, offset_y);
                    cr.scale(scale, scale);

                    // Draw the surface
                    cr.set_source_surface(surface, 0.0, 0.0).unwrap();
                    cr.paint().unwrap();
                    cr.restore().unwrap();
                }
            } else {
                let bg = area.style_context().lookup_color("window_bg_color");
                if let Some(color) = bg {
                    cr.set_source_rgba(
                        color.red() as f64,
                        color.green() as f64,
                        color.blue() as f64,
                        color.alpha() as f64,
                    );
                } else {
                    cr.set_source_rgb(0.15, 0.15, 0.15);
                }
                cr.paint().unwrap();

                let fg = area.style_context().lookup_color("window_fg_color");
                if let Some(color) = fg {
                    cr.set_source_rgba(
                        color.red() as f64,
                        color.green() as f64,
                        color.blue() as f64,
                        color.alpha() as f64,
                    );
                } else {
                    cr.set_source_rgb(0.5, 0.5, 0.5);
                }
                cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                cr.set_font_size(16.0);

                let text = "No image loaded";
                let extents = cr.text_extents(text).unwrap();
                cr.move_to(
                    (width as f64 - extents.width()) / 2.0,
                    (height as f64 + extents.height()) / 2.0,
                );
                cr.show_text(text).unwrap();
            }
        });
    }

    // Wrap preview in a frame for better visual separation
    let preview_frame = gtk4::Frame::builder().child(&preview_area).build();

    // UI layout with modern Libadwaita widgets
    let open_button = gtk4::Button::from_icon_name("document-open-symbolic");
    open_button.set_tooltip_text(Some("Open Image"));

    let save_button = gtk4::Button::from_icon_name("document-save-symbolic");
    save_button.set_tooltip_text(Some("Export as OBJ"));

    // Create the WindowTitle
    let window_title = adw::WindowTitle::new("Shadowpuppet", "");
    let window_title = Rc::new(RefCell::new(window_title));

    let header = adw::HeaderBar::builder()
        .title_widget(&*window_title.borrow())
        .build();

    // Header config
    header.pack_start(&open_button);
    header.pack_end(&save_button);

    // Create a preferences group for layer controls with modern styling
    let layers_label = gtk4::Label::builder()
        .label("Depth Layers")
        .halign(gtk4::Align::Start)
        .build();

    let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 2.0, 64.0, 1.0);
    slider.set_value(8.0);
    slider.set_draw_value(false);
    slider.set_hexpand(true);

    // Create a SpinButton for numeric entry
    let spin_button = gtk4::SpinButton::with_range(2.0, 64.0, 1.0);
    spin_button.set_value(8.0);
    spin_button.set_digits(0);
    spin_button.set_width_chars(3);

    // Create a horizontal box for slider and spin button
    let slider_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .hexpand(true)
        .build();

    slider_box.append(&slider);
    slider_box.append(&spin_button);
    slider_box.set_valign(gtk4::Align::Center);
    slider.set_valign(gtk4::Align::Center);
    spin_button.set_valign(gtk4::Align::Center);

    // Create an action row for the layer control
    let layers_row = adw::ActionRow::builder()
        .title("Depth Layers")
        .subtitle("Number of depth levels in the output")
        .build();

    layers_row.add_suffix(&slider_box);

    // Create preferences group
    let preferences_group = adw::PreferencesGroup::builder()
        .title("Output Settings")
        .build();

    preferences_group.add(&layers_row);

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

    // Use AdwClamp for better responsive design
    let preview_clamp = adw::Clamp::builder()
        .maximum_size(800)
        .tightening_threshold(600)
        .child(&preview_frame)
        .build();

    // Content box with proper spacing
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(24)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    content.append(&preview_clamp);
    content.append(&preferences_group);

    // Add scrolled window for better handling of smaller screens
    let scrolled_window = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(&content)
        .build();

    toast_overlay.set_child(Some(&scrolled_window));

    // Use AdwToolbarView for proper GNOME app structure
    let toolbar_view = adw::ToolbarView::builder().content(&toast_overlay).build();

    toolbar_view.add_top_bar(&header);

    // Use AdwApplicationWindow instead of ApplicationWindow
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Shadowpuppet")
        .default_width(500)
        .default_height(700)
        .content(&toolbar_view)
        .build();

    window.set_size_request(-1, 500);

    let file_chooser_ref = Rc::new(RefCell::new(None::<gtk4::FileChooserNative>));

    // Open button handler
    {
        let app = app.clone();
        let img_data = img_data.clone();
        let preview_area = preview_area.clone();
        let file_chooser_ref = file_chooser_ref.clone();
        let window_title = window_title.clone();
        let toast_overlay = toast_overlay.clone();
        let cached_surface = cached_surface.clone();

        open_button.connect_clicked(move |_| {
            let file_chooser = gtk4::FileChooserNative::builder()
                .title("Open Image")
                .action(FileChooserAction::Open)
                .accept_label("Open")
                .build();

            // Add image file filters
            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Image files"));
            filter.add_mime_type("image/*");
            filter.add_pattern("*.png");
            filter.add_pattern("*.jpg");
            filter.add_pattern("*.jpeg");
            filter.add_pattern("*.bmp");
            filter.add_pattern("*.gif");
            filter.add_pattern("*.webp");
            file_chooser.add_filter(&filter);

            let filter_all = gtk4::FileFilter::new();
            filter_all.set_name(Some("All files"));
            filter_all.add_pattern("*");
            file_chooser.add_filter(&filter_all);

            file_chooser.set_transient_for(Some(&app.active_window().unwrap()));

            *file_chooser_ref.borrow_mut() = Some(file_chooser.clone());

            file_chooser.connect_response({
                let img_data = img_data.clone();
                let preview_area = preview_area.clone();
                let file_chooser_ref = file_chooser_ref.clone();
                let window_title = window_title.clone();
                let toast_overlay = toast_overlay.clone();
                let cached_surface = cached_surface.clone();

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

                                match image::open(&path) {
                                    Ok(img) => {
                                        *img_data.borrow_mut() = Some(img.to_rgb8());
                                        // Clear the cache when loading a new image
                                        *cached_surface.borrow_mut() = None;
                                        preview_area.queue_draw();

                                        let toast = adw::Toast::new("Image loaded successfully");
                                        toast_overlay.add_toast(toast);
                                    }
                                    Err(e) => {
                                        let toast = adw::Toast::new(&format!(
                                            "Failed to load image: {}",
                                            e
                                        ));
                                        toast.set_timeout(5);
                                        toast_overlay.add_toast(toast);
                                    }
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
        let toast_overlay = toast_overlay.clone();
        let window = window.clone();

        save_button.connect_clicked(move |_| {
            if let Some(ref img) = *img_data.borrow() {
                let file_chooser = gtk4::FileChooserNative::builder()
                    .title("Export as OBJ")
                    .action(FileChooserAction::Save)
                    .accept_label("Export")
                    .build();

                // Add OBJ file filter
                let filter = gtk4::FileFilter::new();
                filter.set_name(Some("OBJ files"));
                filter.add_pattern("*.obj");
                file_chooser.add_filter(&filter);

                let filter_all = gtk4::FileFilter::new();
                filter_all.set_name(Some("All files"));
                filter_all.add_pattern("*");
                file_chooser.add_filter(&filter_all);

                file_chooser.set_current_name("output.obj");
                file_chooser.set_transient_for(Some(&window));

                let img_clone = img.clone();
                let layers = *num_layers.borrow();
                let toast_overlay = toast_overlay.clone();

                file_chooser.connect_response(move |dialog, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = dialog.file() {
                            if let Some(path) = file.path() {
                                match save_as_obj(&img_clone, layers, path.to_str().unwrap()) {
                                    Ok(_) => {
                                        let toast =
                                            adw::Toast::new("OBJ file exported successfully");
                                        toast_overlay.add_toast(toast);
                                    }
                                    Err(e) => {
                                        let toast =
                                            adw::Toast::new(&format!("Failed to export: {}", e));
                                        toast.set_timeout(5);
                                        toast_overlay.add_toast(toast);
                                    }
                                }
                            }
                        }
                    }
                    dialog.destroy();
                });

                file_chooser.show();
            } else {
                let toast = adw::Toast::new("Please load an image first");
                toast.set_timeout(3);
                toast_overlay.add_toast(toast);
            }
        });
    }

    // ctrl + q close keybind
    let quit_action = SimpleAction::new("quit", None);
    {
        let window = window.clone();
        quit_action.connect_activate(move |_, _| {
            window.close();
        });
    }
    window.add_action(&quit_action);

    // ctrl + o open keybind
    let open_action = SimpleAction::new("open", None);
    {
        let open_button = open_button.clone();
        open_action.connect_activate(move |_, _| {
            open_button.emit_clicked();
        });
    }
    window.add_action(&open_action);

    // ctrl + s save keybind
    let save_action = SimpleAction::new("save", None);
    {
        let save_button = save_button.clone();
        save_action.connect_activate(move |_, _| {
            save_button.emit_clicked();
        });
    }
    window.add_action(&save_action);

    app.set_accels_for_action("win.quit", &["<Control>q"]);
    app.set_accels_for_action("win.open", &["<Control>o"]);
    app.set_accels_for_action("win.save", &["<Control>s"]);

    window.present();
}

// Save grayscale .obj with error handling
fn save_as_obj(img: &RgbImage, layers: u8, path: &str) -> std::io::Result<()> {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let scale = 0.1;
    let base_height = 0.0;
    let mut file = File::create(path)?;

    let layer_scale = 1.0 / (layers as f32 - 1.0);

    writeln!(file, "mtllib material.mtl\nusemtl plane_material\nnewmtl plane_material\nKd 1.0 1.0 1.0\nKa 0.0 0.0 0.0")?;

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x as u32, y as u32);
            let gray = 0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32;
            let quantized = (gray / 255.0 * (layers as f32 - 1.0)).round() * layer_scale;
            let z = quantized * scale + base_height;
            // Negate the Y coordinate to rotate 180 degrees around X axis
            writeln!(file, "v {} {} {}", x as f32 * scale, -(y as f32 * scale), z)?;
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
            writeln!(file, "f {} {} {} {}", v1, v4, v3, v2)?;
        }
    }

    Ok(())
}
