# GNOME MCP Server

Grant the AI octopus access to a portion of your desktop with a Model Context Protocol (MCP) server for the GNOME desktop.

## Building

To build the project, simply use:

```bash
cargo build
```

This will produce a debug binary under `target/debug/gnome-mcp-server`.

## Usage

To use this MCP server, you need to configure an MCP client. The configuration varies slightly from client to client, but this is the information that you will need:
- Transport: `stdio`
- Command: `/path/to/gnome-mcp-server/target/debug/gnome-mcp-server`
- Args: `[]` (empty)

## Clients

The following is a list of general-purpose MCP clients known to work on Linux (in alphabetical order):

| Name | Description | Open Source | Local LLM Support | Documentation |
|------|-------------|-------------|-------------------|---------------|
| [goose](https://github.com/block/goose) | AI agent by Block (creators of Square) | ✅ | ✅ | [Docs](https://block.github.io/goose/docs/getting-started/using-extensions) |
| [LM Studio](https://lmstudio.ai/) | Desktop app for running local LLMs | ❌ | ✅ | [Docs](https://lmstudio.ai/docs/app/plugins/mcp) |
| [Speed of Light](https://github.com/zugaldia/speedoflight) | Native GNOME MCP client | ✅ | ✅ | [Docs](https://github.com/zugaldia/speedoflight/blob/main/README.md) |

> **Know of other MCP clients that work on Linux?** Please submit a PR to add them to this table.

Coding-specific agents like Claude Code, OpenAI Codex, Gemini CLI, VS Code Copilot, or Cursor also support MCP, but they typically rely on cloud models and have limited or no support for on-device LLMs.

For additional MCP clients, see [this section](https://modelcontextprotocol.io/clients) in the official documentation.

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

#### Contacts
```json
"contacts": {
  "email_only": false         // Include only contacts with emails (default: false)
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

#### `keyring`
- **action** (string, required): store, retrieve, delete
- **label** (string, optional): Human-readable label for the secret (required for store action)
- **secret** (string, optional): The secret value to store (required for store action)
- **attributes** (string, optional): JSON object of key-value attributes for categorizing/searching secrets (e.g., `{"application": "myapp", "username": "user"}`)

**Examples**:
```json
// Store a secret
{"action": "store", "label": "GitHub Token", "secret": "ghp_xxx", "attributes": "{\"service\": \"github\", \"user\": \"myuser\"}"}

// Retrieve by service
{"action": "retrieve", "attributes": "{\"service\": \"github\"}"}

// Delete by user
{"action": "delete", "attributes": "{\"user\": \"myuser\"}"}
```

### Tool/Resource Enabling
- Include config section to enable: `"calendar": {}`
- Omit section to disable
- Empty objects use defaults

See `gnome-mcp-config.example.json` for a complete example.

---

*Co-authored by Claude*
