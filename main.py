import requests
import json
import subprocess
import sys

class YT_API():
    def __init__(self, key):
        self.key = key
        self.url = "https://www.googleapis.com/youtube/v3/"
    def playlist_items(self, pid):
        result = requests.get(self.url + "playlistItems", {
            "key": self.key,
            "part": "contentDetails,id",
            "playlistId": pid,
            "maxResults": 2000
        })
        return json.loads(result.text)
    def videos(self, vid):
        result = requests.get(self.url + "videos", {
            "key": self.key,
            "part": "snippet,id",
            "id": vid,
        })
        return json.loads(result.text)

def main(playlist_id, key, pathto):
    api = YT_API(key)
    pitems = api.playlist_items(playlist_id)["items"]
    m3u8list = ""
    for i in pitems:
        v = api.videos(i["contentDetails"]["videoId"])["items"][0]
        url = "https://youtu.be/" + v['id']
        filename = f"{v['snippet']['title']} - {v['snippet']['channelTitle']}"  
        ext = "mkv"
        subprocess.call(["yt-dlp", url, "--merge-output-format", ext, "-o", f'{pathto}/{filename}.%(ext)s'])
        m3u8list += f"{filename}.{ext}\n"

    with open(f"{pathto}list.m3u8", "w", encoding="UTF-8") as f:
        f.write(m3u8list)

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3])