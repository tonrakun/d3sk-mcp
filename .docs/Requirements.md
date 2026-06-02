# d3sk-mcp 要件定義書 v3

## 概要E

AIエージェントがPCおよびブラウザを操作するため�EMCPサーバ�E、E 
ト�Eクン消費を最小化した軽量レスポンス設計、E

- **pc**: OSレベルのPC操作（�Eウス・キーボ�Eド�Eファイル・アプリ�E�E
- **web**: CDP経由のブラウザ操作（画面非占有�Eバックグラウンド動作！E

---

## v2からの変更点

| # | 変更冁E�� |
|---|---|
| 1 | `batch`に`on_error`オプション追加�E�失敗時の挙動を�E示�E�E|
| 2 | `screenshot`に`scale` / `quality`オプション追加�E�レスポンスサイズ削減！E|
| 3 | `get_interactive`を廁E��し`get_dom`に`interactive_only`フラグで統吁E|
| 4 | `shell`レスポンスをJSON固定に変更�E�区刁E��斁E��衝突回避�E�E|
| 5 | `file_list`レスポンスをJSON固定に変更�E�ファイル名�E特殊文字対策！E|
| 6 | 全webチE�Eルに`session_id?`追加�E��EルチセチE��ョン対応！E|
| 7 | `mouse_click`の`button` / `double`を`action`に統吁E|
| 8 | スクロールに`unit`追加�E�Ex / lines�E�E|
| 9 | `navigate`レスポンスを`"ok"`のみに簡略匁E|
| 10 | エラーをコード化・`get_last_error`チE�Eル追加 |

---

## 技術スタチE��

| 頁E�� | 冁E�� |
|---|---|
| 言誁E| Rust |
| MCPフレームワーク | rmcp |
| PC操佁E| enigo |
| スクリーンショチE�� | xcap |
| ブラウザ操佁E| chromiumoxide�E�EDP�E�E|
| 対象OS | Windows / Mac / Linux |
| 対象ブラウザ | Chrome / Chromium / Brave / Edge�E�Ehromiumベ�Eス�E�E|

---

## 共通仕槁E

- レスポンスは `"ok"` また�E `"E<code>"` を基本とする
- エラー詳細が忁E��な場合�E `get_last_error` で取得すめE
- スクリーンショチE��は `"<base64>,<width>,<height>"` のカンマ区刁E��斁E���Eで返す
- ファイル名�Eコマンド�E力など特殊文字を含む可能性があるレスポンスはJSONで返す
- pc / web どちら�EチE�Eルも単一サーバ�Eから提供すめE

---

## エラーコード一覧

| コーチE| 意味 |
|---|---|
| `E400` | 引数不正 |
| `E404` | 要素・ファイル・プロセスが見つからなぁE|
| `E408` | タイムアウチE|
| `E409` | セチE��ョン競吁E|
| `E500` | 冁E��エラー |

---

## 共通ツール

### [x] `batch`
褁E��チE�EルめEリクエストで連続実行する、E

```
Request:  {
  steps: [{ tool: string, ...args }],
  on_error: "stop" | "continue"  // チE��ォルチE "stop"
}
Response: ["ok", "ok", "E404", ...]  // 各stepの結果配�E
```

### [x] `get_last_error`
直前�Eエラーの詳細メチE��ージを取得する、E

```
Request:  {}
Response: "<detail message>"
```

---

## pc チE�Eル仕槁E

### スクリーンショチE��

#### [x] `screenshot`
```
Request:  {
  target: "screen" | "window",
  window_title?: string,
  scale?: f32,    // 0.1、E.0 チE��ォルチE 1.0
  quality?: u8    // 1、E00 チE��ォルチE 80
}
Response: "<base64>,<width>,<height>"
```

---

### マウス操佁E

#### [x] `mouse_move`
```
Request:  { x: i32, y: i32 }
Response: "ok" | "E<code>"
```

#### [x] `mouse_click`
```
Request:  { x: i32, y: i32, action: "single" | "double" | "right" | "middle" }
Response: "ok" | "E<code>"
```

#### [x] `mouse_scroll`
```
Request:  { x: i32, y: i32, delta_x: i32, delta_y: i32, unit: "px" | "lines" }
Response: "ok" | "E<code>"
```

#### [x] `mouse_drag`
```
Request:  { from_x: i32, from_y: i32, to_x: i32, to_y: i32 }
Response: "ok" | "E<code>"
```

---

### キーボ�Eド操佁E

#### [x] `key_press`
```
Request:  { keys: string[] }  // 侁E ["ctrl", "c"]
Response: "ok" | "E<code>"
```

#### [x] `type_text`
```
Request:  { text: string }
Response: "ok" | "E<code>"
```

---

### クリチE�Eボ�EチE

#### [x] `clipboard_get`
```
Request:  {}
Response: "<text>"
```

#### [x] `clipboard_set`
```
Request:  { text: string }
Response: "ok" | "E<code>"
```

---

### ファイル操佁E

#### [x] `file_read`
```
Request:  { path: string, encoding?: "utf8" | "base64" }
Response: "<content>"
```

#### [x] `file_write`
```
Request:  { path: string, content: string, encoding?: "utf8" | "base64" }
Response: "ok" | "E<code>"
```

#### [x] `file_list`
```
Request:  { path: string, recursive?: bool }
Response: [{ name: string, is_dir: bool, size: u64 }]  // JSON
```

#### [x] `file_delete`
```
Request:  { path: string }
Response: "ok" | "E<code>"
```

#### [x] `file_move`
```
Request:  { from: string, to: string }
Response: "ok" | "E<code>"
```

---

### アプリ操佁E

#### [x] `app_launch`
```
Request:  { command: string, args?: string[] }
Response: "<pid>"
```

#### [x] `app_list`
```
Request:  {}
Response: "<pid>,<name>,<title>\n..."  // 改行区刁E��
```

#### [x] `app_focus`
```
Request:  { pid?: u32, title?: string }
Response: "ok" | "E<code>"
```

#### [x] `app_close`
```
Request:  { pid: u32, force?: bool }
Response: "ok" | "E<code>"
```

---

### シェル

#### [x] `shell`
```
Request:  { cmd: string, timeout_ms?: u64 }
Response: { stdout: string, stderr: string, exit_code: i32 }  // JSON
```

---

## web チE�Eル仕槁E

> CDPを使用するためブラウザは `--remote-debugging-port` 付きで起動する、E 
> 既存�Eロファイル・Cookie・セチE��ョンをそのまま引き継ぐ、E 
> マウスカーソルは一刁E��かなぁE��めPC操作と並行実行可能、E 
> `session_id`省略時�EチE��ォルトセチE��ョンを使用する、E

### 接続�E起勁E

#### [x] `browser_connect`
```
Request:  { port?: u16, browser?: "chrome" | "brave" | "edge" }
Response: "<session_id>"
```

#### [x] `browser_open`
```
Request:  { browser?: "chrome" | "brave" | "edge", profile?: string }
Response: "<session_id>"
```

---

### ナビゲーション

#### [x] `navigate`
```
Request:  { url: string, wait?: "load" | "domcontentloaded" | "networkidle", session_id?: string }
Response: "ok" | "E<code>"
```

#### [x] `get_url`
```
Request:  { session_id?: string }
Response: "<url>,<title>"
```

---

### DOM・コンチE��チE��征E

#### [x] `get_dom`
```
Request:  { selector?: string, depth?: u8, interactive_only?: bool, session_id?: string }
Response: "<text>"
// interactive_only: true の場吁E
// "<tag>,<selector>,<text>\n..." 形式で button/input/select/a のみ返す
```

---

### 要素操佁E

#### [x] `click`
```
Request:  { selector: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### [x] `hover`
```
Request:  { selector: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### [x] `type`
```
Request:  { selector: string, text: string, clear?: bool, session_id?: string }
Response: "ok" | "E<code>"
```

#### [x] `select`
```
Request:  { selector: string, value: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### [x] `check`
```
Request:  { selector: string, checked: bool, session_id?: string }
Response: "ok" | "E<code>"
```

#### [x] `scroll`
```
Request:  { selector?: string, delta_x?: i32, delta_y?: i32, unit: "px" | "lines", session_id?: string }
Response: "ok" | "E<code>"
```

---

### 征E��E

#### [x] `wait_for`
```
Request:  { selector: string, timeout_ms?: u64, state?: "visible" | "hidden", session_id?: string }
Response: "ok" | "E<code>"
```

---

### スクリーンショチE��

#### [x] `web_screenshot`
```
Request:  { full_page?: bool, selector?: string, scale?: f32, quality?: u8, session_id?: string }
Response: "<base64>,<width>,<height>"
```

---

### JavaScript実衁E

#### [x] `evaluate`
```
Request:  { script: string, session_id?: string }
Response: "<result>"
```

---

### タブ管琁E

#### [x] `tab_list`
```
Request:  { session_id?: string }
Response: "<id>,<url>,<title>\n..."  // 改行区刁E��
```

#### [x] `tab_new`
```
Request:  { url?: string, session_id?: string }
Response: "<id>"
```

#### [x] `tab_switch`
```
Request:  { id: string, session_id?: string }
Response: "ok" | "E<code>"
```

#### [x] `tab_close`
```
Request:  { id: string, session_id?: string }
Response: "ok" | "E<code>"
```

---

### Cookie

#### [x] `cookie_get`
```
Request:  { url?: string, name?: string, session_id?: string }
Response: "<name>,<value>,<domain>\n..."  // 改行区刁E��
```

---

## スコープ夁E

- Firefox / Safari 対忁E
- スクリーン録画
- OCR
