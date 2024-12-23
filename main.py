import subprocess
import sys
import urllib.parse
import yt_api

def playlist_url_to_id(url):
    parsed_url = urllib.parse.urlparse(url)
    if((parsed_url.netloc == "youtube.com" or parsed_url.netloc == "www.youtube.com") and parsed_url.path == "/playlist"):   
        query = urllib.parse.parse_qs(parsed_url.query)
        if("list" in query):
            playlist_id = query["list"]
        else:
            playlist_id = "#_err1"
    else:
        playlist_id = "#_err0"
    return playlist_id

def split_by_fifty(id_list):
    splitted_list = []
    c = 0
    for i in id_list:
        splitted_list.append(id_list[c:c+50:1])
        c += 50
        if c >= len(id_list):
            break
    return splitted_list

def get(playlist_id, key, directory):
    api = yt_api.YT_API(key)
    pitems = api.playlist_items(playlist_id)["items"]
    ids = list(map(lambda i: i["contentDetails"]["videoId"], pitems))

    ext = "mkv"
    with open(f"{directory}/list.m3u8", "a", encoding="UTF-8") as f:
        for i in split_by_fifty(ids):
            v = api.videos(",".join(i))["items"]
            for j in v:
                url = f"https://youtu.be/{j['id']}"
                filename = f"{j['snippet']['title']} - {j['snippet']['channelTitle']}.{ext}"
                subprocess.call(["yt-dlp", url, "--merge-output-format", ext, "-o", f'{directory}/{filename}'])
                f.write(f"{filename}\n")

def main(url, key, directory):
    playlist_id = playlist_url_to_id(sys.argv[1])
    if(playlist_id == "#_err0"):
        print("Error: URL is incorrect.")
    elif(playlist_id == "#_err1"):
        print("Error: Parameter is incorrect.")
    else:
        get(playlist_id, sys.argv[2], sys.argv[3])

if __name__ == "__main__":
    if(len(sys.argv) == 4):
        main(sys.argv[1], sys.argv[2], sys.argv[3])
    elif(len(sys.argv) == 1):
        print("reel_system - ")
        print("A software written in Python (uv + pypy environment) to download all videos in the playlist in .mkv format and to generate playlist file in .m3u8 format.")
        print()
        print("Usage: uv run main.py [Playlist URL] [API Key] [Directory]")
    else:
        print("Error: Arguments is incorrect.")