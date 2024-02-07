use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{Error, Visitor};

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MmcPath {
  path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FTBPath {
  path: Option<PathBuf>,
}

impl Display for MmcPath {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.path.clone().unwrap_or_default())
  }
}

impl Display for FTBPath {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.path.clone().unwrap_or_default())
  }
}

impl Default for MmcPath {
  fn default() -> Self {
    let home = dirs::home_dir();

    #[cfg(target_os = "linux")]
    let path = home.map(|home| home.join(".local/share/PrismLauncher/"));

    Self {
      path: path.filter(|path| path.exists()),
    }
  }
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

impl MmcPath {
  pub fn new(path: &str) -> Self {
    Self {
      path: validate_mmc_path(Some(path.into())),
    }
  }

  pub fn get_validated_path(&self) -> Option<PathBuf> {
    validate_mmc_path(self.path.clone())
  }
}

impl FTBPath {
  pub fn new(path: &str) -> Self {
    Self {
      path: validate_ftb_path(Some(path.into())),
    }
  }

  pub fn get_validated_path(&self) -> Option<PathBuf> {
    self.path.clone().filter(|path| path.exists())
  }
}

fn validate_mmc_path(path: Option<PathBuf>) -> Option<PathBuf> {
  path.filter(|path| path.exists())
}

fn validate_ftb_path(path: Option<PathBuf>) -> Option<PathBuf> {
  path.filter(|path| path.exists())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
  pub mmc: MmcPath,
  pub ftb: FTBPath,
}

#[derive(Debug, Deserialize)]
pub struct FTBInstance {
  pub uuid: String,
  pub name: String,
  pub version: String,
  #[serde(rename = "mcVersion")]
  pub mc_version: String,
  #[serde(rename = "modLoader")]
  pub mod_loader: ModLoader,
}

#[derive(Debug)]
pub enum ModLoader {
  Fabric(String),
  Forge(String),
}

#[derive(Debug)]
pub struct ModLoaderVisitor;

impl Visitor<'_> for ModLoaderVisitor {
  type Value = ModLoader;

  fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
    write!(formatter, "a string")
  }

  fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
  where
    E: Error,
  {
    match () {
      // Format: fabric-loader-{mc-version}-{fabric-version}
      _ if v.starts_with("fabric-loader") => v
        .split('-')
        .last()
        .map(|v| ModLoader::Fabric(v.into()))
        .ok_or_else(|| E::custom("couldn't find fabric version")),
      // Format: {mc-version}-forge-{forge-version}
      _ if v.contains("forge") => v
        .split('-')
        .last()
        .map(|v| ModLoader::Forge(v.into()))
        .ok_or_else(|| E::custom("couldn't find forge version")),
      _ => Err(E::custom("no mod loader type found")),
    }
  }
}

impl<'de> Deserialize<'de> for ModLoader {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_str(ModLoaderVisitor)
  }
}

impl FTBInstance {
  pub fn display_name(&self) -> String {
    let uuid = self.uuid.split('-').next().unwrap_or_default();
    format!("{} [{uuid}...]", self.name)
  }
}

fn gen_mmc_instance_cfg(instance: &FTBInstance, has_icon: bool) -> String {
  let mut str = String::new();

  str.push_str("InstanceType=OneSix\n");
  str.push_str("JoinServerOnLaunch=false\n");
  str.push_str("OverrideCommands=false\n");
  str.push_str("OverrideConsole=false\n");
  str.push_str("OverrideGameTime=false\n");
  str.push_str("OverrideJavaArgs=false\n");
  str.push_str("OverrideJavaLocation=false\n");
  str.push_str("OverrideMemory=false\n");
  str.push_str("OverrideNativeWorkarounds=false\n");
  str.push_str("OverrideWindow=false\n");

  if has_icon {
    str.push_str(format!("iconKey={}\n", instance.uuid).as_str());
  } else {
    str.push_str("iconKey=");
  }

  str.push_str(format!("name={}\n", instance.name).as_str());
  str.push_str("notes=\n");

  str
}

fn gen_mmc_pack_json(instance: &FTBInstance) -> serde_json::Value {
  let minecraft_component = |instance: &FTBInstance| {
    serde_json::json!({
      "cachedName": "Minecraft",
      "cachedRequires": [],
      "cachedVersion": instance.mc_version,
      "important": true,
      "uid": "net.minecraft",
      "version": instance.mc_version
    })
  };

  let version_component = |instance: &FTBInstance| match &instance.mod_loader {
    ModLoader::Fabric(version) => serde_json::json!({
      "cachedName": "Fabric Loader",
      "uid": "net.fabricmc.fabric-loader",
      "version": version
    }),
    ModLoader::Forge(version) => serde_json::json!({
      "cachedName": "Forge",
      "uid": "net.minecraftforge",
      "version": version
    }),
  };

  serde_json::json!(
    {
      "components": [
        minecraft_component(instance),
        version_component(instance)
      ],
      "formatVersion": 1
    }
  )
}

#[derive(Debug, Default)]
struct MmcPaths {
  mmc_path: PathBuf,
  mmc_instance_path: PathBuf,
  mmc_icons_path: PathBuf,
}

fn parse_mmc_path(mmc_path: &MmcPath) -> anyhow::Result<MmcPaths> {
  let mmc_path = mmc_path
    .get_validated_path()
    .ok_or_else(|| anyhow::format_err!("invalid mmc path '{:?}'", mmc_path.get_validated_path()))?;

  let cfg = mmc_path.join("prismlauncher.cfg");

  let cfg = if !cfg.exists() {
    mmc_path.join("multimc.cfg")
  } else {
    cfg
  };

  if !cfg.exists() {
    return Err(anyhow::format_err!("mmc config not found"));
  }

  let file = std::fs::read_to_string(cfg)?;

  let mut paths = MmcPaths::default();

  for line in file.lines() {
    let Some((key, value)) = line.split_once('=') else {
      continue;
    };

    match key {
      "InstanceDir" => paths.mmc_instance_path = mmc_path.join(value),
      "IconsDir" => paths.mmc_icons_path = mmc_path.join(value),
      _ => continue,
    }
  }

  paths.mmc_path = mmc_path;

  Ok(paths)
}

pub fn load_ftb_instances(ftb_path: &FTBPath) -> Vec<FTBInstance> {
  let Some(ftb_path) = ftb_path.get_validated_path() else {
    return vec![];
  };

  let Ok(dirs) = ftb_path.read_dir() else {
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

pub fn is_ftb_instance_linked(
  mmc_path: &MmcPath,
  ftb_path: &FTBPath,
  instance: &FTBInstance,
) -> bool {
  let Ok(MmcPaths {
    mmc_instance_path, ..
  }) = parse_mmc_path(mmc_path)
  else {
    return false;
  };

  let Some(ftb_path) = ftb_path.get_validated_path() else {
    return false;
  };

  let path = mmc_instance_path
    .join(instance.uuid.as_str())
    .join(".minecraft");

  let Ok(dest) = std::fs::read_link(path) else {
    return false;
  };

  dest == ftb_path.join(instance.uuid.as_str())
}

pub fn remove_mmc_instance(
  mmc_path: &MmcPath,
  ftb_path: &FTBPath,
  instance: &FTBInstance,
) -> anyhow::Result<()> {
  let MmcPaths {
    mmc_instance_path, ..
  } = parse_mmc_path(mmc_path)?;

  let Some(ftb_path_v) = ftb_path.get_validated_path() else {
    return Err(anyhow::format_err!("failed to validate ftb path"));
  };

  let mmc_instance_path = mmc_instance_path.join(instance.uuid.as_str());
  let mmc_pack_json_file = mmc_instance_path.join("mmc-pack.json");
  let instance_cfg_file = mmc_instance_path.join("instance.cfg");
  let mmc_minecraft_path = mmc_instance_path.join(".minecraft");
  let ftb_instance_path = ftb_path_v.join(instance.uuid.as_str());

  if !is_ftb_instance_linked(mmc_path, ftb_path, instance) {
    return Err(anyhow::format_err!("'{ftb_instance_path:?}' does not link to '{mmc_minecraft_path:?}'"));
  }

  symlink::remove_symlink_dir(mmc_minecraft_path)?;
  std::fs::remove_file(mmc_pack_json_file)?;
  std::fs::remove_file(instance_cfg_file)?;

  let _ = std::fs::remove_dir(mmc_instance_path);

  Ok(())
}

pub fn create_mmc_instance(
  mmc_path: &MmcPath,
  ftb_path: &FTBPath,
  instance: &FTBInstance,
) -> anyhow::Result<()> {
  let MmcPaths {
    mmc_instance_path,
    mmc_icons_path,
    ..
  } = parse_mmc_path(mmc_path)?;

  let Some(ftb_path) = ftb_path.get_validated_path() else {
    return Err(anyhow::format_err!("failed to validate ftb path"));
  };

  let mmc_instance_path = mmc_instance_path.join(instance.uuid.as_str());
  let mmc_icons_path = mmc_icons_path.join(format!("{}.jpg", instance.uuid.as_str()));
  let ftb_instance_path = ftb_path.join(instance.uuid.as_str());

  let ftb_folder_icon = ftb_instance_path.join("folder.jpg");

  let mmc_pack_json = gen_mmc_pack_json(instance);
  let instance_cfg = gen_mmc_instance_cfg(instance, ftb_folder_icon.exists());

  let mmc_pack_json_bytes = serde_json::to_vec_pretty(&mmc_pack_json)?;
  let instance_cfg_bytes = instance_cfg.as_bytes();

  let mmc_pack_json_file = mmc_instance_path.join("mmc-pack.json");
  let instance_cfg_file = mmc_instance_path.join("instance.cfg");

  std::fs::create_dir(&mmc_instance_path)?;
  std::fs::copy(ftb_folder_icon, mmc_icons_path)?;

  std::fs::write(mmc_pack_json_file, mmc_pack_json_bytes)?;
  std::fs::write(instance_cfg_file, instance_cfg_bytes)?;

  let mmc_path = mmc_instance_path.join(".minecraft");

  // std::fs::create_dir(mmc_minecraft)?;
  symlink::symlink_dir(ftb_instance_path, mmc_path)?;

  Ok(())
}
