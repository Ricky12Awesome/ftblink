use std::fmt::Formatter;
use std::fs::create_dir;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Visitor};

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

fn gen_mmc_instance_cfg(instance: &FTBInstance) -> String {
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
  str.push_str("iconKey=default\n");
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
    _ => serde_json::json!({}),
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

    let instance_cfg = dbg!(gen_mmc_instance_cfg(&instance));
    let mmc_pack = dbg!(gen_mmc_pack_json(&instance));

    // let Ok(_) = create_dir(&path) else {
    //   return;
    // };

    let path = path.join("mmc-pack.json");
  }
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

impl FTBInstance {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
  pub mmc: MmcPath,
  pub ftb: FTBPath,
}
