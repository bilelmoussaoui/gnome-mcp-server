use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::mcp::{ResourceProvider, ToolProvider};

// Resource-specific configuration structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    /// Number of days to look ahead for events
    #[serde(default)]
    pub days_ahead: u32,
    /// Number of days to look behind for events
    #[serde(default)]
    pub days_behind: u32,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            days_ahead: 30,
            days_behind: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksConfig {
    /// Include completed tasks in results
    #[serde(default)]
    pub include_completed: bool,
    /// Include cancelled tasks in results
    #[serde(default)]
    pub include_cancelled: bool,
    /// Only show tasks due within X days (0 = all tasks)
    #[serde(default)]
    pub due_within_days: u32,
}

impl Default for TasksConfig {
    fn default() -> Self {
        Self {
            include_completed: true,
            include_cancelled: false,
            due_within_days: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemInfoConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplicationsResourceConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioResourceConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationsConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplicationsToolConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenFileConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WallpaperConfig {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioToolConfig {
    /// Default volume step for relative changes
    #[serde(default)]
    pub volume_step: u32,
}

impl Default for AudioToolConfig {
    fn default() -> Self {
        Self { volume_step: 10 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuickSettingsConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScreenshotConfig {
    /// Show interactive dialog by default
    #[serde(default)]
    pub interactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowManagementConfig {}

// Container structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesConfig {
    pub system_info: Option<SystemInfoConfig>,
    pub applications: Option<ApplicationsResourceConfig>,
    pub calendar: Option<CalendarConfig>,
    pub tasks: Option<TasksConfig>,
    pub audio: Option<AudioResourceConfig>,
}

impl Default for ResourcesConfig {
    fn default() -> Self {
        Self {
            system_info: Some(SystemInfoConfig::default()),
            applications: Some(ApplicationsResourceConfig::default()),
            calendar: Some(CalendarConfig::default()),
            tasks: Some(TasksConfig::default()),
            audio: Some(AudioResourceConfig::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    pub notifications: Option<NotificationsConfig>,
    pub applications: Option<ApplicationsToolConfig>,
    pub open_file: Option<OpenFileConfig>,
    pub wallpaper: Option<WallpaperConfig>,
    pub audio: Option<AudioToolConfig>,
    pub quick_settings: Option<QuickSettingsConfig>,
    pub screenshot: Option<ScreenshotConfig>,
    pub window_management: Option<WindowManagementConfig>,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            notifications: Some(NotificationsConfig::default()),
            applications: Some(ApplicationsToolConfig::default()),
            open_file: Some(OpenFileConfig::default()),
            wallpaper: Some(WallpaperConfig::default()),
            audio: Some(AudioToolConfig::default()),
            quick_settings: Some(QuickSettingsConfig::default()),
            screenshot: Some(ScreenshotConfig::default()),
            window_management: Some(WindowManagementConfig::default()),
        }
    }
}

// Main configuration struct
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub resources: ResourcesConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: Config = serde_json::from_str(&content)
            .with_context(|| "Failed to parse config file as JSON")?;

        Ok(config)
    }

    pub fn load_default() -> Result<Self> {
        let mut config_paths = Vec::new();

        // Current directory
        config_paths.push(PathBuf::from("./gnome-mcp-config.json"));

        // User config directory
        config_paths.push(gio::glib::user_config_dir().join("gnome-mcp/config.json"));

        // System config directories
        for system_config_dir in gio::glib::system_config_dirs() {
            config_paths.push(system_config_dir.join("gnome-mcp/config.json"));
        }

        for path in &config_paths {
            if path.exists() {
                return Self::load_from_file(path);
            }
        }

        // No config file found, use defaults
        Ok(Config::default())
    }

    // Resource enablement checks
    pub fn is_resource_enabled<T: ResourceProvider>(&self) -> bool {
        match T::NAME {
            crate::resources::system_info::SystemInfo::NAME => self.resources.system_info.is_some(),
            crate::resources::applications::Applications::NAME => {
                self.resources.applications.is_some()
            }
            crate::resources::calendar::Calendar::NAME => self.resources.calendar.is_some(),
            crate::resources::tasks::Tasks::NAME => self.resources.tasks.is_some(),
            crate::resources::audio::Audio::NAME => self.resources.audio.is_some(),
            _ => true, // Unknown resources are enabled by default
        }
    }

    pub fn is_tool_enabled<T: ToolProvider>(&self) -> bool {
        match T::NAME {
            crate::tools::notifications::Notifications::NAME => self.tools.notifications.is_some(),
            crate::tools::applications::Applications::NAME => self.tools.applications.is_some(),
            crate::tools::open_file::OpenFile::NAME => self.tools.open_file.is_some(),
            crate::tools::wallpaper::Wallpaper::NAME => self.tools.wallpaper.is_some(),
            crate::tools::audio::Volume::NAME | crate::tools::audio::Media::NAME => {
                self.tools.audio.is_some()
            }
            crate::tools::quick_settings::QuickSettings::NAME => {
                self.tools.quick_settings.is_some()
            }
            crate::tools::screenshot::Screenshot::NAME => self.tools.screenshot.is_some(),
            crate::tools::window_management::WindowManagement::NAME => {
                self.tools.window_management.is_some()
            }
            _ => true, // Unknown tools are enabled by default
        }
    }

    // Configuration getters with defaults
    pub fn get_calendar_config(&self) -> CalendarConfig {
        self.resources.calendar.clone().unwrap_or_default()
    }

    pub fn get_tasks_config(&self) -> TasksConfig {
        self.resources.tasks.clone().unwrap_or_default()
    }

    pub fn get_audio_tool_config(&self) -> AudioToolConfig {
        self.tools.audio.clone().unwrap_or_default()
    }

    pub fn get_screenshot_config(&self) -> ScreenshotConfig {
        self.tools.screenshot.clone().unwrap_or_default()
    }
}

// Global config instance
pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::load_default().unwrap_or_default());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_enabled_default() {
        let config = Config::default();
        // All resources should be enabled by default (no config file = everything
        // enabled)
        assert!(config.is_resource_enabled::<crate::resources::calendar::Calendar>());
        assert!(config.is_resource_enabled::<crate::resources::tasks::Tasks>());
        assert!(config.is_resource_enabled::<crate::resources::system_info::SystemInfo>());
    }

    #[test]
    fn test_example_config_file_is_parseable() {
        let example_config = include_str!("../gnome-mcp-config.example.json");
        let config: Config = serde_json::from_str(example_config).unwrap();

        // Verify the example config enables specific resources and tools
        assert!(config.is_resource_enabled::<crate::resources::calendar::Calendar>());
        assert!(config.is_resource_enabled::<crate::resources::tasks::Tasks>());
        assert!(!config.is_resource_enabled::<crate::resources::system_info::SystemInfo>());

        assert!(config.is_tool_enabled::<crate::tools::notifications::Notifications>());
        assert!(config.is_tool_enabled::<crate::tools::audio::Volume>());
        assert!(!config.is_tool_enabled::<crate::tools::wallpaper::Wallpaper>());

        // Verify custom config values
        let calendar_config = config.get_calendar_config();
        assert_eq!(calendar_config.days_ahead, 60);
        assert_eq!(calendar_config.days_behind, 7);
    }

    #[test]
    fn test_config_parsing() {
        let json = r#"{
            "resources": {
                "calendar": {
                    "days_ahead": 60,
                    "days_behind": 7
                },
                "tasks": {
                    "include_completed": false
                }
            },
            "tools": {
                "notifications": {
                }
            }
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();

        assert!(config.is_resource_enabled::<crate::resources::calendar::Calendar>());
        assert!(config.is_resource_enabled::<crate::resources::tasks::Tasks>());
        assert!(!config.is_resource_enabled::<crate::resources::audio::Audio>());

        let calendar_config = config.get_calendar_config();
        assert_eq!(calendar_config.days_ahead, 60);
        assert_eq!(calendar_config.days_behind, 7);

        let tasks_config = config.get_tasks_config();
        assert!(!tasks_config.include_completed);
        assert!(!tasks_config.include_cancelled);
    }
}
