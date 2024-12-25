import subprocess
import os, sys
import urllib.parse
import yt_api

def playlist_url_to_id(url):
    parsed_url = urllib.parse.urlparse(url)
    LOCATION = parsed_url.netloc
    PATH = parsed_url.path
    if((LOCATION == "youtube.com" or LOCATION == "www.youtube.com") and PATH == "/playlist"):   
        QUERY = urllib.parse.parse_qs(parsed_url.query)
        if("list" in QUERY):
            playlist_id = QUERY["list"][0]
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
        if(c >= len(id_list)):
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
                if("error" in v):
                    url = f"https://youtu.be/{j['id']}"
                    filename = f"{j['snippet']['title']} - {j['snippet']['channelTitle']}"
                    subprocess.call(["yt-dlp", url, "--merge-output-format", ext, "-o", f'{directory}/{filename}.%(ext)s'])
                    if(os.path.exists(f"{directory}/{filename}.mp4")):
                        subprocess.call(["ffmpeg", "-i", f"{directory}/{filename}.mp4", "-c:v", "copy", "-c:a", "copy", f"{directory}/{filename}.mkv"])
                        os.remove(f"{directory}/{filename}.mp4")
                    elif(os.path.exists(f"{directory}/{filename}.webm")):
                        subprocess.call(["ffmpeg", "-i", f"{directory}/{filename}.webm", "-c:v", "copy", "-c:a", "copy", f"{directory}/{filename}.mkv"])
                        os.remove(f"{directory}/{filename}.webm")
                    f.write(f"{filename}.{ext}\n")

def main():
    if(len(sys.argv) == 4):
        playlist_id = playlist_url_to_id(sys.argv[1])
        if(playlist_id == "#_err0"):
            print("Prompt Error: URL is incorrect.")
        elif(playlist_id == "#_err1"):
            print("Prompt Error: Parameter is incorrect.")
        else:
            get(playlist_id, sys.argv[2], sys.argv[3])
    elif(len(sys.argv) == 1):
        print("reel_system - ")
        print("A software written in Python (uv + pypy environment) to download all videos in the playlist in .mkv format and to generate playlist file in .m3u8 format.")
        print()
        print("Usage: uv run main.py [Playlist URL] [API Key] [Directory]")
    else:
        print("Prompt Error: Arguments is incorrect.")

if __name__ == "__main__":
    main()