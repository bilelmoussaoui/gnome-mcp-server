# GNOME MCP Server

A Model Context Protocol (MCP) server for GNOME desktop integration.

## Configuration

By default, all tools and resources are enabled. Create a configuration file to customize behavior:

### Config File Locations

1. `./gnome-mcp-config.json` (current directory)
2. `~/.config/gnome-mcp/config.json` (user config)
3. `/etc/gnome-mcp/config.json` (system config)

### Enable/Disable Tools and Resources

- **Enable**: Include the tool/resource in the config file
- **Disable**: Remove the tool/resource from the config file entirely

Example config:
```json
{
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
}
```

See `gnome-mcp-config.example.json` for a complete example.
