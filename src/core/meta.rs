use crate::prelude::*;
use std::collections::VecDeque;

const FILE: &str = "meta.json";

#[derive(Serialize, Deserialize)]
pub struct Meta {
  pub cmds: VecDeque<String>,
}
impl ::std::default::Default for Meta {
  fn default() -> Self {
    Self {
      cmds: VecDeque::new(),
    }
  }
}
// testing this
impl Meta {
  pub fn load() -> Result<Self> {
    let json = match fs::read_to_string(FILE) {
      Ok(json) => json,
      Err(_) => return Ok(Self::default()),
    };
    Ok(serde_json::from_str(&json)?)
  }
  fn save(&self) -> Result<()> {
    let _ = fs::write(FILE, serde_json::to_string(self)?);
    Ok(())
  }
  pub fn log_command(cmd: impl AsRef<str>) -> Result<()> {
    let mut meta = Self::load()?;
    let cmd = cmd.as_ref();

    // do not double log commands
    if let Some(front) = meta.cmds.front() {
      if front == cmd {
        return Ok(());
      }
    }

    meta.cmds.push_front(cmd.into());
    meta.cmds.truncate(100);
    meta.save()?;
    Ok(())
  }
}
