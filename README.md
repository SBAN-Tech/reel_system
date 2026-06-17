# reel_system

Rust で書かれた、YouTube プレイリスト内の動画を `.mkv` 形式でダウンロードし、`list.m3u8` を生成する CLI ツールです。

## 使い方

あらかじめ `yt-dlp` と `ffmpeg` を PATH に通しておいてください。

```powershell
cargo run -- "[Playlist URL]" "[YouTube Data API Key]" "[Directory]"
```

並列数を指定する場合:

```powershell
cargo run -- "[Playlist URL]" "[YouTube Data API Key]" "[Directory]" 4
```

第4引数の並列数は省略できます。省略時は利用可能な CPU 数を使います。

ビルド済みバイナリを使う場合:

```powershell
cargo build --release
.\target\release\reel_system.exe "[Playlist URL]" "[YouTube Data API Key]" "[Directory]" 4
```

`Directory` の中に動画ファイルと `list.m3u8` が生成されます。動画のダウンロードと `.mkv` への変換は並列で処理され、`list.m3u8` はプレイリスト順で書き出されます。
