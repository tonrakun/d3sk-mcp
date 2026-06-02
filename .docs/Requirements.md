# d3sk-mcp 要件定義書 v4

## 概要

AIエージェントがPCおよびブラウザを操作するためのMCPサーバー。
トークン消費を最小化した軽量レスポンス設計。

- **pc**: OSレベルのPC操作（マウス・キーボード・ファイル・アプリ）
- **web**: CDP経由のブラウザ操作（画面非占有・バックグラウンド動作）

---

## v4 変更点（v3 → v4）

| # | 変更内容 |
|---|---|
| 1 | `ok/e400/e404/e408/e500` ヘルパーを `common.rs` に集約（pc・web の重複定義を削除） |
| 2 | 未使用の `D3skError` enum を削除 |
| 3 | 未使用の `winapi` クレートを削除 |
| 4 | `mouse_click` / `mouse_drag` の `std::thread::sleep` を `tokio::time::sleep` に変更 |
| 5 | `batch` のエラー判定を `starts_with('E')` から `is_error()` による厳密な照合に変更 |
| 6 | `get_url` / `tab_list` / `cookie_get` / `app_list` のレスポンスをJSON化（カンマ区切り廃止） |
| 7 | `get_dom` の `depth` パラメータを実装（JS深さ制限シリアライザ） |
| 8 | `web_screenshot` の `scale` パラメータを実装（Lanczos3リサイズ） |
| 9 | `browser_close` ツールを追加 |
| 10 | `navigate` の `wait` 挙動を明確化（`load` / `domcontentloaded` は `goto` で完結、`networkidle` のみ追加待機） |

---

## v3 変更点（v2 → v3）

| # | 変更内容 |
|---|---|
| 1 | `batch` に `on_error` オプション追加（失敗時の挙動を制御） |
| 2 | `screenshot` に `scale` / `quality` オプション追加（レスポンスサイズ削減） |
| 3 | `get_interactive` を廃止し `get_dom` に `interactive_only` フラグで統合 |
| 4 | `shell` レスポンスをJSON固定に変更（区切り文字衝突回避） |
| 5 | `file_list` レスポンスをJSON固定に変更（ファイル名の特殊文字対策） |
| 6 | 全webツールに `session_id?` 追加（マルチセッション対応） |
| 7 | `mouse_click` の `button` / `double` を `action` に統合 |
| 8 | スクロールに `unit` 追加（px / lines） |
| 9 | `navigate` レスポンスを `"ok"` のみに簡略化 |
| 10 | エラーをコード化・`get_last_error` ツール追加 |

---

## 技術スタック

| 項目 | 内容 |
|---|---|
| 言語 | Rust |
| MCPフレームワーク | rmcp |
| PC操作 | enigo |
| スクリーンショット | xcap |
| ブラウザ操作 | chromiumoxide（CDP） |
| 対象OS | Windows / Mac / Linux |
| 対象ブラウザ | Chrome / Chromium / Brave / Edge（Chromiumベース） |

---

## 共通仕様

- レスポンスは `"ok"` または `"E<code>"` を基本とする
- エラー詳細が必要な場合は `get_last_error` で取得する
- スクリーンショットは `"<base64>,<width>,<height>"` のカンマ区切り形式で返す
- 特殊文字を含む可能性があるレスポンス（ファイル名・URL・クッキー値など）はJSONで返す
- pc / web どちらのツールも単一サーバーから提供する

---

## エラーコード一覧

| コード | 意味 |
|---|---|
| `E400` | 引数不正 |
| `E404` | 要素・ファイル・プロセス・セッションが見つからない |
| `E408` | タイムアウト |
| `E409` | セッション競合 |
| `E500` | 内部エラー |

---

## 共通ツール

### `batch`
複数ツールを1リクエストで連続実行する。

```
Request:  {
  steps_json: string,  // JSON配列文字列 [{ tool: string, ...args }]
  on_error?: "stop" | "continue"  // デフォルト "stop"
}
Response: ["ok", "ok", "E404", ...]  // 各stepの結果配列（JSON）
```

### `get_last_error`
直前のエラーの詳細メッセージを取得する。

```
Request:  {}
Response: "<detail message>"
```

---

## pc ツール仕様

### スクリーンショット

#### `screenshot`
```
Request:  {
  target: "screen" | "window",
  window_title?: string,
  scale?: f32,    // 0.1〜1.0  デフォルト 1.0
  quality?: u8    // 1〜100  デフォルト 80
}
Response: "<base64>,<width>,<height>"
```

---

### マウス操作

#### `mouse_move`
```
Request:  { x: i32, y: i32 }
Response: "ok" | "E<code>"
```

#### `mouse_click`
```
Request:  { x: i32, y: i32, action: "single" | "double" | "right" | "middle" }
Response: "ok" | "E<code>"
```

#### `mouse_scroll`
```
Request:  { x: i32, y: i32, delta_x: i32, delta_y: i32, unit: "px" | "lines" }
Response: "ok" | "E<code>"
```

#### `mouse_drag`
```
Request:  { from_x: i32, from_y: i32, to_x: i32, to_y: i32 }
Response: "ok" | "E<code>"
```

---

### キーボード操作

#### `key_press`
```
Request:  { keys: string[] }  // 例: ["ctrl", "c"]
Response: "ok" | "E<code>"
```

#### `type_text`
```
Request:  { text: string }
Response: "ok" | "E<code>"
```

---

### クリップボード

#### `clipboard_get`
```
Request:  {}
Response: "<text>"
```

#### `clipboard_set`
```
Request:  { text: string }
Response: "ok" | "E<code>"
```

---

### ファイル操作

#### `file_read`
```
Request:  { path: string, encoding?: "utf8" | "base64" }
Response: "<content>"
```

#### `file_write`
```
Request:  { path: string, content: string, encoding?: "utf8" | "base64" }
Response: "ok" | "E<code>"
```

#### `file_list`
```
Request:  { path: string, recursive?: bool }
Response: [{ name: string, is_dir: bool, size: u64 }]  // JSON
```

#### `file_delete`
```
Request:  { path: string }
Response: "ok" | "E<code>"
```

#### `file_move`
```
Request:  { from: string, to: string }
Response: "ok" | "E<code>"
```

---

### アプリ操作

#### `app_launch`
```
Request:  { command: string, args?: string[] }
Response: "<pid>"
```

#### `app_list`
```
Request:  {}
Response: [{ pid: number, name: string, title: string }]  // JSON
```

#### `app_focus`
```
Request:  { pid?: u32, title?: string }
Response: "ok" | "E<code>"
```

#### `app_close`
```
Request:  { pid: u32, force?: bool }
Response: "ok" | "E<code>"
```

---

### シェル

#### `shell`
```
Request:  { cmd: string, timeout_ms?: u64 }
Response: { stdout: string, stderr: string, exit_code: i32 }  // JSON
```

---

## web ツール仕様

> CDPを使用するためブラウザは `--remote-debugging-port` 付きで起動するか、`browser_open` で起動する。  
> 既存のプロファイル・Cookie・セッションをそのまま引き継ぐ。  
> マウスカーソルは一切動かないためPC操作と並行実行可能。  
> `session_id` 省略時はデフォルトセッションを使用する。

### 接続・起動

#### `browser_connect`
```
Request:  { port?: u16, browser?: "chrome" | "brave" | "edge" }
Response: "<session_id>"
```

#### `browser_open`
```
Request:  { browser?: "chrome" | "brave" | "edge", profile?: string }
Response: "<session_id>"
```

#### `browser_close`
`browser_open` で起動したセッションはブラウザプロセスを終了する。`browser_connect` で接続したセッションは切断のみ。
```
Request:  { session_id?: string }
Response: "ok" | "E<code>"
```

---

### ナビゲーション

#### `navigate`
`wait` の挙動：`goto` は `load` イベントまで待機するため、`"load"` と `"domcontentloaded"` は同等。`"networkidle"` のみ `wait_for_navigation` による追加待機を行う。
```
Request:  { url: string, wait?: "load" | "domcontentloaded" | "networkidle", session_id?: string }
Response: "ok" | "E<code>"
```

#### `get_url`
```
Request:  { session_id?: string }
Response: { url: string, title: string }  // JSON
```

---

### DOM・コンテンツ取得

#### `get_dom`
`depth` を指定した場合は JavaScript による深さ制限シリアライザを使用する（指定深さを超えた子ノードは `<tag>...</tag>` に省略）。
```
Request:  {
  selector?: string,
  depth?: u8,             // 省略時: 全深さ取得
  interactive_only?: bool,
  session_id?: string
}

// interactive_only: false（デフォルト）の場合
Response: "<html>"

// interactive_only: true の場合
// button/input/select/a/textarea のみ返す
Response: "<tag>,<selector>,<text>\n..."

// depth 指定の場合
Response: "<depth-limited html>"
```

---

### 要素操作

#### `click`
```
Request:  { selector: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### `hover`
```
Request:  { selector: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### `type`
```
Request:  { selector: string, text: string, clear?: bool, session_id?: string }
Response: "ok" | "E<code>"
```

#### `select`
```
Request:  { selector: string, value: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### `check`
```
Request:  { selector: string, checked: bool, session_id?: string }
Response: "ok" | "E<code>"
```

#### `scroll`
```
Request:  { selector?: string, delta_x?: i32, delta_y?: i32, unit: "px" | "lines", session_id?: string }
Response: "ok" | "E<code>"
```

---

### 待機

#### `wait_for`
```
Request:  { selector: string, timeout_ms?: u64, state?: "visible" | "hidden", session_id?: string }
Response: "ok" | "E<code>"
```

---

### スクリーンショット

#### `web_screenshot`
`scale` を指定した場合はスクリーンショット取得後にLanczos3でリサイズする（0.1〜2.0）。
```
Request:  { full_page?: bool, scale?: f32, quality?: u8, session_id?: string }
Response: "<base64>,<width>,<height>"
```

---

### JavaScript実行

#### `evaluate`
```
Request:  { script: string, session_id?: string }
Response: "<result>"
```

---

### タブ管理

#### `tab_list`
```
Request:  { session_id?: string }
Response: [{ id: string, url: string, title: string }]  // JSON
```

#### `tab_new`
```
Request:  { url?: string, session_id?: string }
Response: "<id>"
```

#### `tab_switch`
```
Request:  { id: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### `tab_close`
```
Request:  { id: string, session_id?: string }
Response: "ok" | "E<code>"
```

---

### Cookie

#### `cookie_get`
```
Request:  { url?: string, name?: string, session_id?: string }
Response: [{ name: string, value: string, domain: string }]  // JSON
```

---

## スコープ外

- Firefox / Safari 対応
- スクリーン録画
- OCR
