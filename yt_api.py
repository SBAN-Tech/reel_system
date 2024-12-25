import urllib.request
import json
import sys

class YT_API():
    def __init__(self, key):
        self.key = key
        self.url = "https://www.googleapis.com/youtube/v3/"
    def error(self, status):
        if(status == 400):
            print("API Error: API key is incorrect. (400; Bad Request)")
            sys.exit(1)
        if(status == 401):
            print("API Error: Authority is incorrect. (401; Unauthorized)")
            sys.exit(1)
        if(status == 404):
            print("API Error: Playlist or video is not found. (404; Not Found)")
            sys.exit(1)
        if(status == 418):
            print("API Error: Brewed a Coffee in a Teapot. (418; I'm a teapot)")
            sys.exit(1)
    def playlists(self, pid):
        values = {
            "key": self.key,
            "part": "contentDetails",
            "id": pid,
        }
        query = urllib.parse.urlencode(values)
        req = urllib.request.Request(self.url + "playlists?" + query)
        print(self.url + "playlists?" + query)
        try:
            with urllib.request.urlopen(req) as response:
                return json.loads(response.read().decode())
        except urllib.error.HTTPError as e:
            self.error(e.code)
    def playlist_items(self, pid):
        video_total = self.playlists(pid)["items"][0]["contentDetails"]["itemCount"]
        lengthby50 = int(video_total / 50) + 1
        items = []
        page_token = ""
        for i in range(lengthby50):
            values = {
                "key": self.key,
                "part": "contentDetails,id",
                "playlistId": pid,
                "maxResults": 50,
                "pageToken": page_token
            }
            query = urllib.parse.urlencode(values)
            req = urllib.request.Request(self.url + "playlistItems?" + query)
            with urllib.request.urlopen(req) as response:
                jsonized = json.loads(response.read())
                if("nextPageToken" in jsonized):
                    page_token = jsonized["nextPageToken"]
                items += jsonized["items"]
        result = {"kind": jsonized["kind"], "etag": jsonized["etag"], "pageInfo": jsonized["pageInfo"], "items": items}
        return result
    def videos(self, vid):
        values = {
            "key": self.key,
            "part": "snippet,id",
            "id": vid,
        }
        query = urllib.parse.urlencode(values)
        req = urllib.request.Request(self.url + "videos?" + query)
        try:
            with urllib.request.urlopen(req) as response:
                return json.loads(response.read())
        except urllib.error.HTTPError as e:
            self.error(e.code)