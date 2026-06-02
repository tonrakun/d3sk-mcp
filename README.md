# d3sk-mcp

Desktop automation MCP server written in Rust. Gives AI agents full control over the desktop and web browser via the [Model Context Protocol](https://modelcontextprotocol.io/).

## Features

**PC control**
- Screenshots (full screen or specific window)
- Mouse — move, click (single/double/right/middle), scroll, drag
- Keyboard — key combos, text input
- Clipboard — get/set
- File system — read, write, list, delete, move
- App management — launch, list, focus, close
- Shell — run commands with timeout (PowerShell on Windows, sh on Linux/macOS)

**Browser control** (via Chrome DevTools Protocol)
- Connect to a running browser or launch one
- Navigate, get URL/title
- DOM inspection (full HTML or interactive-elements-only)
- Element interaction — click, hover, type, select, check, scroll
- Wait for element visibility
- Screenshots (viewport or full page)
- JavaScript evaluation
- Tab management — list, open, switch, close
- Cookie retrieval

**Batch execution**
- Run multiple tools in a single call with `stop` or `continue` on error

## Requirements

- Rust 1.85+
- For browser tools: Chromium-based browser (Chrome / Brave / Edge) with remote debugging enabled, or let the server launch one

## Build

```sh
cargo build --release
```

The binary is at `target/release/d3sk-mcp`.

## Usage

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "d3sk": {
      "command": "C:/path/to/d3sk-mcp.exe"
    }
  }
}
```

### Claude Code

```sh
claude mcp add d3sk -- /path/to/d3sk-mcp
```

### Browser setup

To use browser tools, either:

**Option A — connect to a running browser** (keeps your session / cookies):

```sh
# Chrome
chrome.exe --remote-debugging-port=9222

# Then call:
# browser_connect { "port": 9222 }
```

**Option B — let the server launch a fresh browser**:

```
browser_open { "browser": "chrome" }
```

## Tools reference

### Common

| Tool | Description |
|------|-------------|
| `batch` | Execute multiple tools in sequence. `steps_json`: JSON array of `{"tool":"...","arg":...}`. `on_error`: `"stop"`\|`"continue"` |
| `get_last_error` | Get the detail message of the last error |

### PC

| Tool | Key params | Returns |
|------|-----------|---------|
| `screenshot` | `target`: `"screen"`\|`"window"`, `window_title`, `scale`, `quality` | `<base64>,<w>,<h>` |
| `mouse_move` | `x`, `y` | `ok` |
| `mouse_click` | `x`, `y`, `action`: `"single"`\|`"double"`\|`"right"`\|`"middle"` | `ok` |
| `mouse_scroll` | `x`, `y`, `delta_x`, `delta_y`, `unit`: `"px"`\|`"lines"` | `ok` |
| `mouse_drag` | `from_x`, `from_y`, `to_x`, `to_y` | `ok` |
| `key_press` | `keys`: e.g. `["ctrl","c"]` | `ok` |
| `type_text` | `text` | `ok` |
| `clipboard_get` | — | clipboard text |
| `clipboard_set` | `text` | `ok` |
| `file_read` | `path`, `encoding`: `"utf8"`\|`"base64"` | file content |
| `file_write` | `path`, `content`, `encoding` | `ok` |
| `file_list` | `path`, `recursive` | JSON `[{name, is_dir, size}]` |
| `file_delete` | `path` | `ok` |
| `file_move` | `from`, `to` | `ok` |
| `app_launch` | `command`, `args` | PID |
| `app_list` | — | `<pid>,<name>,<title>` lines |
| `app_focus` | `pid` or `title` | `ok` |
| `app_close` | `pid`, `force` | `ok` |
| `shell` | `cmd`, `timeout_ms` | JSON `{stdout, stderr, exit_code}` |

### Web

All web tools accept an optional `session_id` (defaults to the first/only session).

| Tool | Key params | Returns |
|------|-----------|---------|
| `browser_connect` | `port` (default 9222) | `session_id` |
| `browser_open` | `browser`: `"chrome"`\|`"brave"`\|`"edge"`, `profile` | `session_id` |
| `navigate` | `url`, `wait`: `"load"`\|`"networkidle"` | `ok` |
| `get_url` | — | `<url>,<title>` |
| `get_dom` | `selector`, `interactive_only` | HTML or `tag,selector,text` lines |
| `click` | `selector` | `ok` |
| `hover` | `selector` | `ok` |
| `type_input` | `selector`, `text`, `clear` | `ok` |
| `web_select` | `selector`, `value` | `ok` |
| `check` | `selector`, `checked` | `ok` |
| `web_scroll` | `selector`, `delta_x`, `delta_y`, `unit` | `ok` |
| `wait_for` | `selector`, `state`: `"visible"`\|`"hidden"`, `timeout_ms` | `ok` |
| `web_screenshot` | `full_page`, `quality` | `<base64>,<w>,<h>` |
| `evaluate` | `script` | JS result as string |
| `tab_list` | — | `<id>,<url>,<title>` lines |
| `tab_new` | `url` | tab id |
| `tab_switch` | `id` | `ok` |
| `tab_close` | `id` | `ok` |
| `cookie_get` | `url`, `name` | `<name>,<value>,<domain>` lines |

## Error codes

All tools return one of these strings on failure. Call `get_last_error` immediately after to get the detail message.

| Code | Meaning |
|------|---------|
| `E400` | Bad arguments |
| `E404` | Not found (file, window, element, session) |
| `E408` | Timeout |
| `E409` | Session conflict |
| `E500` | Internal error |

## License

MIT
