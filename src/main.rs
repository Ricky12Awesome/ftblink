slint::include_modules!();

slint::slint! { import { AppWindow, PathSelector, PathSelectorGlobal } from "ui/appwindow.slint"; }

mod cfg;

fn setup_path_selectors(ui: &AppWindow) {
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
}

fn main() -> Result<(), slint::PlatformError> {
  let ui = AppWindow::new()?;

  setup_path_selectors(&ui);

  ui.on_load_packs(move |s| {
    let ftb = cfg::FTBPath::new(&s);
    let instances = ftb.load_instances();
    let display = instances
      .iter()
      .map(|instance| instance.name.as_str().into())
      .collect::<Vec<slint::SharedString>>();

    display.as_slice().into()
  });

  if let Some(path) = cfg::PrismPath::default().path {
    ui.set_prism_path(path.to_string_lossy().to_string().into());
  }

  if let Some(path) = cfg::FTBPath::default().path {
    let path = path.to_string_lossy().to_string();

    ui.set_ftb_path(path.clone().into());

    let packs = ui.invoke_load_packs(path.into());

    ui.set_packs(packs);
  }


  ui.run()
}
