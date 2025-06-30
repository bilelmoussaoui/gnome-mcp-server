# GNOME MCP Server

Grant the AI octopus access to a portion of your desktop with a Model Context Protocol (MCP) server for the GNOME desktop.

## Configuration

By default, all tools and resources are enabled. Create a configuration file to customize behavior:

### Config File Location
- `./gnome-mcp-config.json` (current directory)
- `~/.config/gnome-mcp/config.json` (user config)
- `/etc/gnome-mcp/config.json` (system config)

### Resources

#### Calendar
```json
"calendar": {
  "days_ahead": 30,    // Days to look ahead (default: 30)
  "days_behind": 0     // Days to look behind (default: 0)
}
```

#### Tasks
```json
"tasks": {
  "include_completed": true,   // Include completed tasks (default: true)
  "include_cancelled": false,  // Include cancelled tasks (default: false)
  "due_within_days": 0        // Filter by due date, 0 = all (default: 0)
}
```

### Tools

#### `send_notification`
- **summary** (string, required): Notification title
- **body** (string, required): Notification content

#### `launch_application`
- **app_name** (string, required): Application name or executable

#### `open_file`
- **path** (string, required): File path or URL

#### `set_wallpaper`
- **image_path** (string, required): Full path to image file
- Supported formats: JPG, JPEG, PNG

#### `set_volume`
- **volume** (number, optional): Volume level 0-100
- **mute** (boolean, optional): Mute/unmute
- **relative** (boolean, optional): Relative change if true
- **direction** (string, optional): "up" or "down" for default step

Config:
```json
"audio": {
  "volume_step": 10    // Default step size (default: 10)
}
```

#### `media_control`
- **action** (string, required): play, pause, play_pause, stop, next, previous
- **player** (string, optional): Specific player name (default: active player)

#### `quick_settings`
- **setting** (string, required): wifi, bluetooth, night_light, do_not_disturb, dark_style
- **enabled** (boolean, required): Enable/disable state

#### `take_screenshot`
- **interactive** (boolean, optional): Show selection dialog

Config:
```json
"screenshot": {
  "interactive": false    // Default interactive mode (default: false)
}
```

#### `window_management`
- **action** (string, required): list, focus, close, minimize, maximize, switch_workspace, move_to_workspace, get_geometry, set_geometry, set_position, set_size, snap
- **window_id** (string, optional): Window ID for window-specific actions
- **workspace** (integer, optional): Workspace number (0-indexed)
- **x** (integer, optional): X coordinate
- **y** (integer, optional): Y coordinate
- **width** (integer, optional): Width in pixels
- **height** (integer, optional): Height in pixels
- **position** (string, optional): "left" or "right" for snap action

**Requirements**: GNOME Shell unsafe mode: `Alt+F2` → `lg` → `global.context.unsafe_mode = true`

### Tool/Resource Enabling
- Include config section to enable: `"calendar": {}`
- Omit section to disable
- Empty objects use defaults

See `gnome-mcp-config.example.json` for a complete example.

---

*Co-authored by Claude*
