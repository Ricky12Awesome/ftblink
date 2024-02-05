slint::include_modules!();

mod path_selector;

slint::slint! { import { AppWindow, PathSelector, PathSelectorGlobal } from "ui/appwindow.slint"; }

fn main() -> Result<(), slint::PlatformError> {
  let ui = AppWindow::new()?;

  let global = ui.global::<PathSelectorGlobal>();

  global.on_open(move |s| {
    let _ = open::that_detached(s.as_str());
  });

  global.on_browse(move |s| {
    let mut dialog = native_dialog::FileDialog::new();

    if !s.is_empty() {
      dialog = dialog.set_location(s.as_str());
    }

    dialog
      .show_open_single_dir()
      .ok()
      .flatten()
      .map(|s| s.to_string_lossy().to_string().into())
      .unwrap_or(s)
  });

  ui.run()
}
