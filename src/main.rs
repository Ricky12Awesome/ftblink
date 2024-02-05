slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
  let ui = AppWindow::new()?;

  ui.on_input_accepted(move |s| {
    println!("{s}");
  });

  ui.run()
}
