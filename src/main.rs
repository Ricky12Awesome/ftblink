#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use slint::{Image, SharedString, Weak};

use ftblink::{
  create_mmc_instance, is_ftb_instance_linked, load_ftb_instances, remove_mmc_instance, Config,
  FTBInstance, FTBPath, MmcPath,
};

slint::include_modules!();

slint::slint! { import { AppWindow, PathSelector, PathSelectorGlobal } from "ui/appwindow.slint"; }

#[derive(Default)]
pub struct AppState {
  ftb_path: Rc<RefCell<FTBPath>>,
  mmc_path: Rc<RefCell<MmcPath>>,
  packs: Rc<RefCell<Vec<FTBInstance>>>,
  ui: Weak<AppWindow>,
}

impl Debug for AppState {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("AppState")
      .field("ftb_path", &self.ftb_path)
      .field("mmc_path", &self.mmc_path)
      .field("packs", &self.packs)
      .finish_non_exhaustive()
  }
}

fn display_packs(packs: &[FTBInstance]) -> Vec<SharedString> {
  packs
    .iter()
    .map(|instance| instance.display_name().into())
    .collect::<Vec<SharedString>>()
}

fn get_pack_icon(ftb_path: &FTBPath, ftb_instance: &FTBInstance) -> Option<Image> {
  ftb_path
    .get_validated_path()
    .map(|ftb_path| ftb_path.join(&ftb_instance.uuid).join("folder.jpg"))
    .filter(|path| path.exists())
    .and_then(|path| Image::load_from_path(&path).ok())
}

impl AppState {
  fn new(ui: Weak<AppWindow>) -> Self {
    Self {
      ui,
      ..Self::default()
    }
  }

  fn load_packs(&self) {
    let ftb = self.ftb_path.borrow();

    let mut packs = self.packs.borrow_mut();
    *packs = load_ftb_instances(&ftb);

    let ui = self.ui.unwrap();
    let display = display_packs(packs.as_slice());

    ui.set_packs(display.as_slice().into());

    if let Some(pack) = display.first() {
      ui.set_selected(0);
      ui.set_selected_display(pack.clone());

      let pack = &packs[0];
      let mmc_path = self.mmc_path.borrow();
      let ftb_path = self.ftb_path.borrow();
      let is_linked = is_ftb_instance_linked(&mmc_path, &ftb_path, pack);

      if let Some(pack_icon) = get_pack_icon(&ftb_path, pack) {
        ui.set_pack_icon_exists(true);
        ui.set_pack_icon(pack_icon);
      }

      ui.set_linked(is_linked);
    }

    if packs.is_empty() {
      ui.set_selected(-1);
      ui.set_selected_display("[none]".into());
      ui.set_linked(false);
      ui.set_pack_icon_exists(false);
    }
  }
}

fn main() -> Result<(), slint::PlatformError> {
  let ui = AppWindow::new()?;
  let state = Rc::new(AppState::new(ui.as_weak()));
  let config = Config::load();

  if let Ok(Config { mmc_path, ftb_path }) = &config {
    if mmc_path.path.is_some() {
      *state.mmc_path.borrow_mut() = mmc_path.clone();
    }

    if ftb_path.path.is_some() {
      *state.ftb_path.borrow_mut() = ftb_path.clone();
    }
  }

  setup_path_selectors(&ui);

  if let Some(path) = state.mmc_path.borrow().get_validated_path() {
    ui.set_mmc_path(path.to_string_lossy().to_string().into());
  }

  if let Some(path) = state.ftb_path.borrow().get_validated_path() {
    ui.set_ftb_path(path.to_string_lossy().to_string().into());
  }

  state.load_packs();

  ui.on_save({
    let state = state.clone();

    move || {
      let ui = state.ui.unwrap();
      let mmc_path = state.mmc_path.borrow().clone();
      let ftb_path = state.ftb_path.borrow().clone();

      match config.as_ref().cloned() {
        Ok(mut config) => {
          if mmc_path.path.is_some() {
            config.mmc_path = mmc_path;
          }

          if ftb_path.path.is_some() {
            config.ftb_path = ftb_path;
          }

          if let Err(err) = config.save() {
            ui.set_error(err.to_string().into());
          }
        }
        Err(_) => {
          let config = Config { mmc_path, ftb_path };

          if let Err(err) = config.save() {
            ui.set_error(err.to_string().into());
          }
        }
      }
    }
  });

  ui.on_selected_change({
    let state = state.clone();

    move |_| {
      let ui = state.ui.unwrap();
      let i = ui.get_selected();
      let packs = state.packs.borrow();

      if let Some(pack) = packs.get(i as usize) {
        let mmc_path = state.mmc_path.borrow();
        let ftb_path = state.ftb_path.borrow();
        let is_linked = is_ftb_instance_linked(&mmc_path, &ftb_path, pack);

        if let Some(pack_icon) = get_pack_icon(&ftb_path, pack) {
          ui.set_pack_icon_exists(true);
          ui.set_pack_icon(pack_icon);
        }

        ui.set_linked(is_linked);
      }
    }
  });

  ui.on_mmc_path_edited({
    let state = state.clone();

    move |s| {
      *state.mmc_path.borrow_mut() = MmcPath::new(&s);
    }
  });

  ui.on_ftb_path_edited({
    let state = state.clone();

    move |s| {
      let mut ftb_path = state.ftb_path.borrow_mut();

      *ftb_path = FTBPath::new(&s);

      drop(ftb_path);

      state.load_packs();
    }
  });

  ui.on_accepted({
    let state = state.clone();

    move || {
      let ui = state.ui.unwrap();
      let mmc_path = state.mmc_path.borrow();
      let ftb_path = state.ftb_path.borrow();
      let selected = ui.get_selected();

      let packs = state.packs.borrow();

      if selected == -1 || selected >= packs.len() as _ {
        ui.set_error("Nothing is selected".into());
        return;
      };

      let Some(pack) = packs.get(selected as usize) else {
        ui.set_error("Selected pack not found".into());
        return;
      };

      let is_linked = is_ftb_instance_linked(&mmc_path, &ftb_path, pack);

      match is_linked {
        true => {
          let result = remove_mmc_instance(&mmc_path, &ftb_path, pack);

          match result {
            Ok(_) => {
              ui.set_linked(!is_linked);
            }
            Err(err) => ui.set_error(err.to_string().into()),
          }
        }
        false => {
          let result = create_mmc_instance(&mmc_path, &ftb_path, pack);

          match result {
            Ok(_) => {
              ui.set_linked(!is_linked);
            }
            Err(err) => ui.set_error(err.to_string().into()),
          }
        }
      }
    }
  });

  ui.run()
}

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
