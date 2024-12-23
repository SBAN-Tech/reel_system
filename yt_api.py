import requests
import json
import sys

class YT_API():
    def __init__(self, key):
        self.key = key
        self.url = "https://www.googleapis.com/youtube/v3/"
    def error(self, status):
        if(status == 400):
            print("API Error: API key is incorrect. (400)")
            sys.exit(1)
        if(status == 404):
            print("API Error: Playlist is not found. (404)")
            sys.exit(1)
    def playlists(self, pid):
        response = requests.get(self.url + "playlistItems", {
            "key": self.key,
            "part": "contentDetails",
            "playlistId": pid,
        })
        self.error(response.status_code)
        return json.loads(response.text)
    def playlist_items(self, pid):
        video_total = self.playlists(pid)["pageInfo"]["totalResults"]
        lengthby50 = int(video_total / 50) + 1
        items = []
        page_token = ""
        for i in range(lengthby50):
            response = requests.get(self.url + "playlistItems", {
                "key": self.key,
                "part": "contentDetails,id",
                "playlistId": pid,
                "maxResults": 50,
                "pageToken": page_token
            })
            jsonized = json.loads(response.text)
            if("nextPageToken" in jsonized):
                page_token = jsonized["nextPageToken"]
            items += jsonized["items"]
        result = {"kind": jsonized["kind"], "etag": jsonized["etag"], "pageInfo": jsonized["pageInfo"], "items": items}
        return result
    def videos(self, vid):
        response = requests.get(self.url + "videos", {
            "key": self.key,
            "part": "snippet,id",
            "id": vid,
        })
        self.error(response.status_code)
        return json.loads(response.text)