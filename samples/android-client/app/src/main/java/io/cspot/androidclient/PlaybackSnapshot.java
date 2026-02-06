package io.cspot.androidclient;

import org.json.JSONException;
import org.json.JSONObject;

public final class PlaybackSnapshot {
    public static final int STATE_STOPPED = 0;
    public static final int STATE_LOADING = 1;
    public static final int STATE_PLAYING = 2;
    public static final int STATE_PAUSED = 3;

    public final boolean running;
    public final boolean ready;
    public final boolean connected;
    public final int playbackState;
    public final int positionMs;
    public final int durationMs;
    public final int volume;
    public final String statusMessage;
    public final String deviceName;
    public final String title;
    public final String artist;
    public final String album;
    public final String artworkUrl;

    public PlaybackSnapshot(
            boolean running,
            boolean ready,
            boolean connected,
            int playbackState,
            int positionMs,
            int durationMs,
            int volume,
            String statusMessage,
            String deviceName,
            String title,
            String artist,
            String album,
            String artworkUrl) {
        this.running = running;
        this.ready = ready;
        this.connected = connected;
        this.playbackState = playbackState;
        this.positionMs = positionMs;
        this.durationMs = durationMs;
        this.volume = volume;
        this.statusMessage = statusMessage;
        this.deviceName = deviceName;
        this.title = title;
        this.artist = artist;
        this.album = album;
        this.artworkUrl = artworkUrl;
    }

    public static PlaybackSnapshot empty() {
        return new PlaybackSnapshot(
                false,
                false,
                false,
                -1,
                0,
                0,
                0,
                "cspot is idle",
                "",
                "",
                "",
                "",
                "");
    }

    public static PlaybackSnapshot fromJson(String json) {
        if (json == null || json.trim().isEmpty()) {
            return empty();
        }

        try {
            JSONObject object = new JSONObject(json);
            return new PlaybackSnapshot(
                    object.optBoolean("running", false),
                    object.optBoolean("ready", false),
                    object.optBoolean("connected", false),
                    object.optInt("playbackState", -1),
                    object.optInt("positionMs", 0),
                    object.optInt("durationMs", 0),
                    object.optInt("volume", 0),
                    object.optString("statusMessage", ""),
                    object.optString("deviceName", ""),
                    object.optString("title", ""),
                    object.optString("artist", ""),
                    object.optString("album", ""),
                    object.optString("artworkUrl", ""));
        } catch (JSONException ex) {
            return new PlaybackSnapshot(
                    false,
                    false,
                    false,
                    -1,
                    0,
                    0,
                    0,
                    "Failed to parse playback snapshot",
                    "",
                    "",
                    "",
                    "",
                    "");
        }
    }

    public String playbackStateLabel() {
        switch (playbackState) {
            case STATE_STOPPED:
                return "stopped";
            case STATE_LOADING:
                return "loading";
            case STATE_PLAYING:
                return "playing";
            case STATE_PAUSED:
                return "paused";
            default:
                return "invalid";
        }
    }
}
