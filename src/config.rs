use evdev_rs::enums::EV_KEY;
use serde::Deserialize;

use crate::numpad_layout::SupportedLayout;

#[derive(Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case", untagged)]
pub(crate) enum CustomCommand {
    /// Press these keys
    Keys(Vec<EV_KEY>),
    /// Run this command with given args
    Command { cmd: String, args: Vec<String> },
}

impl Default for CustomCommand {
    fn default() -> Self {
        // default is the calculator key
        Self::Keys(vec![EV_KEY::KEY_CALC])
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Deserialize)]
pub(crate) struct Config {
    layout: SupportedLayout,

    #[serde(default = "default_numlock")]
    disable_numlock_on_start: bool,

    #[serde(default)]
    calc_start_command: CustomCommand,

    calc_stop_command: Option<CustomCommand>,
}

fn default_numlock() -> bool {
    true
}

impl Config {
    /// Get a reference to the config's layout.
    pub(crate) fn layout(&self) -> &SupportedLayout {
        &self.layout
    }

    /// Get a reference to the config's disable numlock on start.
    pub(crate) fn disable_numlock_on_start(&self) -> bool {
        self.disable_numlock_on_start
    }

    /// Get a reference to the config's calc start command.
    pub(crate) fn calc_start_command(&self) -> &CustomCommand {
        &self.calc_start_command
    }

    /// Get a reference to the config's calc stop command.
    pub(crate) fn calc_stop_command(&self) -> Option<&CustomCommand> {
        self.calc_stop_command.as_ref()
    }
}
