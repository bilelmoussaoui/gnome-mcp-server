use anyhow::Result;

use crate::{mcp::ToolProvider, tool_params};

async fn eval_shell_script(shell_proxy: &zbus::Proxy<'_>, script: &str) -> Result<String> {
    let response = shell_proxy.call_method("Eval", &(script,)).await
        .map_err(|e| {
            if e.to_string().contains("AccessDenied") || e.to_string().contains("not authorized") {
                anyhow::anyhow!("Access denied. GNOME Shell unsafe mode must be enabled. Open Looking Glass (Alt+F2, type 'lg') and run: global.context.unsafe_mode = true")
            } else {
                anyhow::anyhow!("Shell eval failed: {}", e)
            }
        })?;

    let result: (bool, String) = response.body().deserialize()?;

    if result.0 {
        Ok(result.1)
    } else {
        Err(anyhow::anyhow!("Script execution failed: {}", result.1))
    }
}

#[derive(Default)]
pub struct WindowManagement;

tool_params! {
    WindowManagementParams,
    required(action: string, "Action to perform: 'list', 'focus', 'close', 'minimize', 'maximize', 'switch_workspace', 'move_to_workspace', 'get_geometry', 'set_geometry', 'set_position', 'set_size', 'snap'"),
    optional(window_id: string, "Window ID for focus/close/minimize/maximize/move_to_workspace/geometry actions"),
    optional(workspace: i64, "Workspace number for switch_workspace/move_to_workspace actions (0-based)"),
    optional(x: i64, "X coordinate for set_geometry/set_position actions"),
    optional(y: i64, "Y coordinate for set_geometry/set_position actions"),
    optional(width: i64, "Width for set_geometry/set_size actions"),
    optional(height: i64, "Height for set_geometry/set_size actions"),
    optional(position: string, "Position for snap action: 'left', 'right'")
}

impl ToolProvider for WindowManagement {
    const NAME: &'static str = "window_management";
    const DESCRIPTION: &'static str = "Manage windows and workspaces via GNOME Shell (requires unsafe mode). Actions: list, focus, close, minimize, maximize, switch_workspace, move_to_workspace, get_geometry, set_geometry, set_position, set_size, snap. Note: Workspaces are 0-indexed (workspace 0 is the first workspace, workspace 1 is the second, etc.). You cannot move windows to or switch to workspaces that don't exist yet - GNOME may create workspaces dynamically or use a fixed number depending on user configuration.";
    type Params = WindowManagementParams;

    async fn execute_with_params(&self, params: Self::Params) -> Result<serde_json::Value> {
        Self::execute_with_result(|| execute_window_action(params)).await
    }
}

async fn execute_window_action(params: WindowManagementParams) -> Result<String> {
    let connection = zbus::Connection::session().await?;

    let shell_proxy = zbus::Proxy::new(
        &connection,
        "org.gnome.Shell",
        "/org/gnome/Shell",
        "org.gnome.Shell",
    )
    .await?;

    // Check if we can access GNOME Shell
    let _version: String = shell_proxy
        .get_property("ShellVersion")
        .await
        .map_err(|e| anyhow::anyhow!("Cannot connect to GNOME Shell: {}", e))?;

    match params.action.as_str() {
        "list" => list_windows(&shell_proxy).await,
        "focus" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for focus action"))?;
            focus_window(&shell_proxy, &id).await
        },
        "close" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for close action"))?;
            close_window(&shell_proxy, &id).await
        },
        "minimize" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for minimize action"))?;
            minimize_window(&shell_proxy, &id).await
        },
        "maximize" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for maximize action"))?;
            maximize_window(&shell_proxy, &id).await
        },
        "switch_workspace" => {
            let ws = params.workspace.ok_or_else(|| anyhow::anyhow!("workspace required for switch_workspace action"))?;
            switch_workspace(&shell_proxy, ws as i32).await
        },
        "move_to_workspace" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for move_to_workspace action"))?;
            let ws = params.workspace.ok_or_else(|| anyhow::anyhow!("workspace required for move_to_workspace action"))?;
            move_window_to_workspace(&shell_proxy, &id, ws as i32).await
        },
        "get_geometry" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for get_geometry action"))?;
            get_window_geometry(&shell_proxy, &id).await
        },
        "set_geometry" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for set_geometry action"))?;
            let x_val = params.x.ok_or_else(|| anyhow::anyhow!("x required for set_geometry action"))?;
            let y_val = params.y.ok_or_else(|| anyhow::anyhow!("y required for set_geometry action"))?;
            let w_val = params.width.ok_or_else(|| anyhow::anyhow!("width required for set_geometry action"))?;
            let h_val = params.height.ok_or_else(|| anyhow::anyhow!("height required for set_geometry action"))?;
            set_window_geometry(&shell_proxy, &id, x_val as i32, y_val as i32, w_val as i32, h_val as i32).await
        },
        "set_position" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for set_position action"))?;
            let x_val = params.x.ok_or_else(|| anyhow::anyhow!("x required for set_position action"))?;
            let y_val = params.y.ok_or_else(|| anyhow::anyhow!("y required for set_position action"))?;
            set_window_position(&shell_proxy, &id, x_val as i32, y_val as i32).await
        },
        "set_size" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for set_size action"))?;
            let w_val = params.width.ok_or_else(|| anyhow::anyhow!("width required for set_size action"))?;
            let h_val = params.height.ok_or_else(|| anyhow::anyhow!("height required for set_size action"))?;
            set_window_size(&shell_proxy, &id, w_val as i32, h_val as i32).await
        },
        "snap" => {
            let id = params.window_id.ok_or_else(|| anyhow::anyhow!("window_id required for snap action"))?;
            let pos = params.position.ok_or_else(|| anyhow::anyhow!("position required for snap action"))?;
            snap_window(&shell_proxy, &id, &pos).await
        },
        _ => Err(anyhow::anyhow!("Unknown action: {}. Available: list, focus, close, minimize, maximize, switch_workspace, move_to_workspace, get_geometry, set_geometry, set_position, set_size, snap", params.action)),
    }
}

async fn list_windows(shell_proxy: &zbus::Proxy<'_>) -> Result<String> {
    let script = r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_window_type() === Meta.WindowType.NORMAL && !w.is_skip_taskbar())
            .map(w => ({
                id: w.get_id(),
                title: w.get_title(),
                wm_class: w.get_wm_class(),
                workspace: w.get_workspace().index(),
                minimized: w.minimized,
                maximized: w.get_maximized(),
                focused: w.has_focus()
            }));
        JSON.stringify(windows);
    "#;

    let result = eval_shell_script(shell_proxy, script).await?;
    let windows: serde_json::Value = serde_json::from_str(&result)?;
    Ok(format!(
        "Windows:\n{}",
        serde_json::to_string_pretty(&windows)?
    ))
}

async fn focus_window(shell_proxy: &zbus::Proxy<'_>, window_id: &str) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            window.activate(global.get_current_time());
            'focused';
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}

async fn close_window(shell_proxy: &zbus::Proxy<'_>, window_id: &str) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            window.delete(global.get_current_time());
            'closed';
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}

async fn minimize_window(shell_proxy: &zbus::Proxy<'_>, window_id: &str) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            window.minimize();
            'minimized';
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}

async fn maximize_window(shell_proxy: &zbus::Proxy<'_>, window_id: &str) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            if (window.get_maximized()) {{
                window.unmaximize(Meta.MaximizeFlags.BOTH);
                'unmaximized';
            }} else {{
                window.maximize(Meta.MaximizeFlags.BOTH);
                'maximized';
            }}
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}

async fn switch_workspace(shell_proxy: &zbus::Proxy<'_>, workspace_index: i32) -> Result<String> {
    let script = format!(
        r#"
        let workspaceManager = global.workspace_manager;
        let workspace = workspaceManager.get_workspace_by_index({workspace_index});
        if (workspace) {{
            workspace.activate(global.get_current_time());
            'switched to workspace {workspace_index}';
        }} else {{
            'workspace {workspace_index} not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(result)
}

async fn move_window_to_workspace(
    shell_proxy: &zbus::Proxy<'_>,
    window_id: &str,
    workspace_index: i32,
) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            let workspaceManager = global.workspace_manager;
            let targetWorkspace = workspaceManager.get_workspace_by_index({workspace_index});
            if (targetWorkspace) {{
                window.change_workspace(targetWorkspace);
                'moved to workspace {workspace_index}';
            }} else {{
                'workspace {workspace_index} not found';
            }}
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}

async fn get_window_geometry(shell_proxy: &zbus::Proxy<'_>, window_id: &str) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            let rect = window.get_frame_rect();
            JSON.stringify({{
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height
            }});
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    if result == "window not found" {
        Ok(format!("Window {window_id} not found"))
    } else {
        Ok(format!("Window {window_id} geometry: {result}"))
    }
}

async fn set_window_geometry(
    shell_proxy: &zbus::Proxy<'_>,
    window_id: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            window.unmaximize(Meta.MaximizeFlags.BOTH);
            window.move_resize_frame(false, {x}, {y}, {width}, {height});
            'geometry set';
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}

async fn set_window_position(
    shell_proxy: &zbus::Proxy<'_>,
    window_id: &str,
    x: i32,
    y: i32,
) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            let rect = window.get_frame_rect();
            window.move_resize_frame(false, {x}, {y}, rect.width, rect.height);
            'position set';
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}

async fn set_window_size(
    shell_proxy: &zbus::Proxy<'_>,
    window_id: &str,
    width: i32,
    height: i32,
) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            window.unmaximize(Meta.MaximizeFlags.BOTH);
            let rect = window.get_frame_rect();
            window.move_resize_frame(false, rect.x, rect.y, {width}, {height});
            'size set';
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}

async fn snap_window(
    shell_proxy: &zbus::Proxy<'_>,
    window_id: &str,
    position: &str,
) -> Result<String> {
    let script = format!(
        r#"
        let windows = global.get_window_actors()
            .map(w => w.get_meta_window())
            .filter(w => w.get_id() === {window_id});
        if (windows.length > 0) {{
            let window = windows[0];
            let monitor = window.get_monitor();
            let workArea = global.workspace_manager.get_active_workspace().get_work_area_for_monitor(monitor);

            window.unmaximize(Meta.MaximizeFlags.BOTH);

            let x, y, width, height;
            if ('{position}' === 'left') {{
                x = workArea.x;
                y = workArea.y;
                width = Math.floor(workArea.width / 2);
                height = workArea.height;
            }} else if ('{position}' === 'right') {{
                x = workArea.x + Math.floor(workArea.width / 2);
                y = workArea.y;
                width = Math.ceil(workArea.width / 2);
                height = workArea.height;
            }} else {{
                'invalid position: must be left or right';
            }}

            if (x !== undefined) {{
                window.move_resize_frame(false, x, y, width, height);
                'snapped to {position}';
            }} else {{
                'invalid position: must be left or right';
            }}
        }} else {{
            'window not found';
        }}
    "#
    );

    let result = eval_shell_script(shell_proxy, &script).await?;
    Ok(format!("Window {window_id} {result}"))
}
