# reel_system

再生リストにある動画を全て`.mkv`形式でダウンロードするプログラムのRust/Cargo実装です。

## ビルド
`cargo run --release`でビルド可能です。

## 使用時のこころがけ
- 使用時は再生リストのURLをコマンドライン引数に渡しておき、yt-dlpとffmpegをPATHに通しておいてください。
    - 引数と順番: 再生リストURL→YouTube Data APIのAPIキー→ディレクトリ→並列スレッド
    - YouTube Data APIのAPIキー以降は引数は必須ではありませんが、デフォルトで環境変数`REEL_SYSTEM_API_KEY`を参照します。
    - ディレクトリ未指定時は実行ディレクトリに出力されます。
- VLCには、ディレクトリをドロップアウトせずディレクトリ中に生成されるlist.m3u8を再生してください。