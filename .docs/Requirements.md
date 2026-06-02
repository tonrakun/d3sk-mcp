# d3sk-mcp 要件定義書 v3

## 概要

AIエージェントがPCおよびブラウザを操作するためのMCPサーバー。  
トークン消費を最小化した軽量レスポンス設計。

- **pc**: OSレベルのPC操作（マウス・キーボード・ファイル・アプリ）
- **web**: CDP経由のブラウザ操作（画面非占有・バックグラウンド動作）

---

## v2からの変更点

| # | 変更内容 |
|---|---|
| 1 | `batch`に`on_error`オプション追加（失敗時の挙動を明示） |
| 2 | `screenshot`に`scale` / `quality`オプション追加（レスポンスサイズ削減） |
| 3 | `get_interactive`を廃止し`get_dom`に`interactive_only`フラグで統合 |
| 4 | `shell`レスポンスをJSON固定に変更（区切り文字衝突回避） |
| 5 | `file_list`レスポンスをJSON固定に変更（ファイル名の特殊文字対策） |
| 6 | 全webツールに`session_id?`追加（マルチセッション対応） |
| 7 | `mouse_click`の`button` / `double`を`action`に統合 |
| 8 | スクロールに`unit`追加（px / lines） |
| 9 | `navigate`レスポンスを`"ok"`のみに簡略化 |
| 10 | エラーをコード化・`get_last_error`ツール追加 |

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
- スクリーンショットは `"<base64>,<width>,<height>"` のカンマ区切り文字列で返す
- ファイル名・コマンド出力など特殊文字を含む可能性があるレスポンスはJSONで返す
- pc / web どちらのツールも単一サーバーから提供する

---

## エラーコード一覧

| コード | 意味 |
|---|---|
| `E400` | 引数不正 |
| `E404` | 要素・ファイル・プロセスが見つからない |
| `E408` | タイムアウト |
| `E409` | セッション競合 |
| `E500` | 内部エラー |

---

## 共通ツール

### [ ] `batch`
複数ツールを1リクエストで連続実行する。

```
Request:  {
  steps: [{ tool: string, ...args }],
  on_error: "stop" | "continue"  // デフォルト: "stop"
}
Response: ["ok", "ok", "E404", ...]  // 各stepの結果配列
```

### [ ] `get_last_error`
直前のエラーの詳細メッセージを取得する。

```
Request:  {}
Response: "<detail message>"
```

---

## pc ツール仕様

### スクリーンショット

#### [ ] `screenshot`
```
Request:  {
  target: "screen" | "window",
  window_title?: string,
  scale?: f32,    // 0.1〜1.0 デフォルト: 1.0
  quality?: u8    // 1〜100 デフォルト: 80
}
Response: "<base64>,<width>,<height>"
```

---

### マウス操作

#### [ ] `mouse_move`
```
Request:  { x: i32, y: i32 }
Response: "ok" | "E<code>"
```

#### [ ] `mouse_click`
```
Request:  { x: i32, y: i32, action: "single" | "double" | "right" | "middle" }
Response: "ok" | "E<code>"
```

#### [ ] `mouse_scroll`
```
Request:  { x: i32, y: i32, delta_x: i32, delta_y: i32, unit: "px" | "lines" }
Response: "ok" | "E<code>"
```

#### [ ] `mouse_drag`
```
Request:  { from_x: i32, from_y: i32, to_x: i32, to_y: i32 }
Response: "ok" | "E<code>"
```

---

### キーボード操作

#### [ ] `key_press`
```
Request:  { keys: string[] }  // 例: ["ctrl", "c"]
Response: "ok" | "E<code>"
```

#### [ ] `type_text`
```
Request:  { text: string }
Response: "ok" | "E<code>"
```

---

### クリップボード

#### [ ] `clipboard_get`
```
Request:  {}
Response: "<text>"
```

#### [ ] `clipboard_set`
```
Request:  { text: string }
Response: "ok" | "E<code>"
```

---

### ファイル操作

#### [ ] `file_read`
```
Request:  { path: string, encoding?: "utf8" | "base64" }
Response: "<content>"
```

#### [ ] `file_write`
```
Request:  { path: string, content: string, encoding?: "utf8" | "base64" }
Response: "ok" | "E<code>"
```

#### [ ] `file_list`
```
Request:  { path: string, recursive?: bool }
Response: [{ name: string, is_dir: bool, size: u64 }]  // JSON
```

#### [ ] `file_delete`
```
Request:  { path: string }
Response: "ok" | "E<code>"
```

#### [ ] `file_move`
```
Request:  { from: string, to: string }
Response: "ok" | "E<code>"
```

---

### アプリ操作

#### [ ] `app_launch`
```
Request:  { command: string, args?: string[] }
Response: "<pid>"
```

#### [ ] `app_list`
```
Request:  {}
Response: "<pid>,<name>,<title>\n..."  // 改行区切り
```

#### [ ] `app_focus`
```
Request:  { pid?: u32, title?: string }
Response: "ok" | "E<code>"
```

#### [ ] `app_close`
```
Request:  { pid: u32, force?: bool }
Response: "ok" | "E<code>"
```

---

### シェル

#### [ ] `shell`
```
Request:  { cmd: string, timeout_ms?: u64 }
Response: { stdout: string, stderr: string, exit_code: i32 }  // JSON
```

---

## web ツール仕様

> CDPを使用するためブラウザは `--remote-debugging-port` 付きで起動する。  
> 既存プロファイル・Cookie・セッションをそのまま引き継ぐ。  
> マウスカーソルは一切動かないためPC操作と並行実行可能。  
> `session_id`省略時はデフォルトセッションを使用する。

### 接続・起動

#### [ ] `browser_connect`
```
Request:  { port?: u16, browser?: "chrome" | "brave" | "edge" }
Response: "<session_id>"
```

#### [ ] `browser_open`
```
Request:  { browser?: "chrome" | "brave" | "edge", profile?: string }
Response: "<session_id>"
```

---

### ナビゲーション

#### [ ] `navigate`
```
Request:  { url: string, wait?: "load" | "domcontentloaded" | "networkidle", session_id?: string }
Response: "ok" | "E<code>"
```

#### [ ] `get_url`
```
Request:  { session_id?: string }
Response: "<url>,<title>"
```

---

### DOM・コンテンツ取得

#### [ ] `get_dom`
```
Request:  { selector?: string, depth?: u8, interactive_only?: bool, session_id?: string }
Response: "<text>"
// interactive_only: true の場合
// "<tag>,<selector>,<text>\n..." 形式で button/input/select/a のみ返す
```

---

### 要素操作

#### [ ] `click`
```
Request:  { selector: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### [ ] `hover`
```
Request:  { selector: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### [ ] `type`
```
Request:  { selector: string, text: string, clear?: bool, session_id?: string }
Response: "ok" | "E<code>"
```

#### [ ] `select`
```
Request:  { selector: string, value: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### [ ] `check`
```
Request:  { selector: string, checked: bool, session_id?: string }
Response: "ok" | "E<code>"
```

#### [ ] `scroll`
```
Request:  { selector?: string, delta_x?: i32, delta_y?: i32, unit: "px" | "lines", session_id?: string }
Response: "ok" | "E<code>"
```

---

### 待機

#### [ ] `wait_for`
```
Request:  { selector: string, timeout_ms?: u64, state?: "visible" | "hidden", session_id?: string }
Response: "ok" | "E<code>"
```

---

### スクリーンショット

#### [ ] `web_screenshot`
```
Request:  { full_page?: bool, selector?: string, scale?: f32, quality?: u8, session_id?: string }
Response: "<base64>,<width>,<height>"
```

---

### JavaScript実行

#### [ ] `evaluate`
```
Request:  { script: string, session_id?: string }
Response: "<result>"
```

---

### タブ管理

#### [ ] `tab_list`
```
Request:  { session_id?: string }
Response: "<id>,<url>,<title>\n..."  // 改行区切り
```

#### [ ] `tab_new`
```
Request:  { url?: string, session_id?: string }
Response: "<id>"
```

#### [ ] `tab_switch`
```
Request:  { id: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### [ ] `tab_close`
```
Request:  { id: string, session_id?: string }
Response: "ok" | "E<code>"
```

---

### Cookie

#### [ ] `cookie_get`
```
Request:  { url?: string, name?: string, session_id?: string }
Response: "<name>,<value>,<domain>\n..."  // 改行区切り
```

---

## スコープ外

- Firefox / Safari 対応
- スクリーン録画
- OCR