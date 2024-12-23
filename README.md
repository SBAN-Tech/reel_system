# reel_system(仮称)
Python(uv + pypy環境)で書かれた再生リストにある動画を全て`.mkv`形式でダウンロードするプログラムです。
## 使用法
あらかじめuvをインストールしておき、`uv run main.py [再生リストURL] [APIキー] [ダウンロードする動画を入れるディレクトリ]`と実行します。
## 使用時のこころがけ
- 使用時は再生リストのURL，YouTube Data APIのAPIキー，ディレクトリの順番にコマンドライン引数に渡しておき、yt-dlpをPATHに通しておいてください。
- ディレクトリ文字列の最後に`/`は入れない方がいいです。
- VLCには、ディレクトリをドロップアウトせずディレクトリ中に生成されるlist.m3u8を再生してください。