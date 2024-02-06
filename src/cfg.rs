use std::fs::create_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FTBPath {
  pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MmcPath {
  pub path: Option<PathBuf>,
}

impl Default for FTBPath {
  fn default() -> Self {
    let home = dirs::home_dir();

    #[cfg(target_os = "linux")]
    let path = home.map(|home| home.join(".ftba/instances/"));

    Self {
      path: path.filter(|path| path.exists()),
    }
  }
}

impl Default for MmcPath {
  fn default() -> Self {
    let home = dirs::home_dir();

    #[cfg(target_os = "linux")]
    let path = home.map(|home| home.join(".local/share/PrismLauncher/instances"));

    Self {
      path: path.filter(|path| path.exists()),
    }
  }
}

impl FTBPath {
  pub fn new(path: &str) -> Self {
    Self {
      path: Some(PathBuf::from(path)).filter(|path| path.exists()),
    }
  }

  pub fn load_instances(&self) -> Vec<FTBInstance> {
    let Some(path) = self.path.as_ref().filter(|path| path.exists()) else {
      return vec![];
    };

    let Ok(dirs) = path.read_dir() else {
      return vec![];
    };

    dirs
      .filter_map(|dir| dir.ok())
      .map(|dir| dir.path().join("instance.json"))
      .filter(|instance| instance.exists())
      .filter_map(|instance| std::fs::read_to_string(instance).ok())
      .filter_map(|instance| serde_json::from_str::<FTBInstance>(&instance).ok())
      .collect()
  }
}

impl MmcPath {
  pub fn new(path: &str) -> Self {
    Self {
      path: Some(PathBuf::from(path)).filter(|path| path.exists()),
    }
  }

  pub fn create_instance(&self, instance: FTBInstance) {
    let Some(path) = self.path.as_ref().filter(|path| path.exists()) else {
      return;
    };

    let path = path.join(instance.name.as_str());

    let Ok(_) = create_dir(&path) else {
      return;
    };

    let path = path.join("mmc-pack.json");

  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FTBInstance {
  pub uuid: String,
  pub name: String,
  pub version: String,
  #[serde(rename = "mcVersion")]
  pub mc_version: String,
  #[serde(rename = "modLoader")]
  pub mod_loader: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
  pub mmc: MmcPath,
  pub ftb: FTBPath,
}
