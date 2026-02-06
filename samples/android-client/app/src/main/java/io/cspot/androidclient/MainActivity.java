package io.cspot.androidclient;

import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.widget.Button;
import android.widget.ImageView;
import android.widget.SeekBar;
import android.widget.TextView;
import android.widget.Toast;

import androidx.appcompat.app.AppCompatActivity;

import java.io.IOException;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.Locale;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public class MainActivity extends AppCompatActivity {
    private static final int POLL_INTERVAL_MS = 1000;

    private final Handler refreshHandler = new Handler(Looper.getMainLooper());
    private final ExecutorService artworkExecutor = Executors.newSingleThreadExecutor();

    private TextView statusText;
    private TextView connectionText;
    private TextView titleText;
    private TextView artistText;
    private TextView albumText;
    private TextView timeText;
    private TextView volumeValueText;

    private ImageView artworkImage;

    private SeekBar positionSeekBar;
    private SeekBar volumeSeekBar;

    private Button playPauseButton;

    private PlaybackSnapshot latestSnapshot = PlaybackSnapshot.empty();
    private boolean userSeeking = false;
    private boolean userAdjustingVolume = false;
    private String currentArtworkUrl = "";

    private final Runnable refreshTask = new Runnable() {
        @Override
        public void run() {
            refreshUi();
            refreshHandler.postDelayed(this, POLL_INTERVAL_MS);
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        bindViews();
        setupControls();

        String deviceName = "cspot Android Client (" + Build.MODEL + ")";
        NativeBridge.start(deviceName, getApplicationContext());
    }

    @Override
    protected void onStart() {
        super.onStart();
        refreshHandler.post(refreshTask);
    }

    @Override
    protected void onStop() {
        super.onStop();
        refreshHandler.removeCallbacks(refreshTask);
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        artworkExecutor.shutdownNow();
    }

    private void bindViews() {
        statusText = findViewById(R.id.statusText);
        connectionText = findViewById(R.id.connectionText);
        titleText = findViewById(R.id.titleText);
        artistText = findViewById(R.id.artistText);
        albumText = findViewById(R.id.albumText);
        timeText = findViewById(R.id.timeText);
        volumeValueText = findViewById(R.id.volumeValueText);

        artworkImage = findViewById(R.id.artworkImage);

        positionSeekBar = findViewById(R.id.positionSeekBar);
        volumeSeekBar = findViewById(R.id.volumeSeekBar);

        playPauseButton = findViewById(R.id.playPauseButton);
    }

    private void setupControls() {
        Button previousButton = findViewById(R.id.previousButton);
        Button nextButton = findViewById(R.id.nextButton);
        Button transferButton = findViewById(R.id.transferButton);

        previousButton.setOnClickListener(view -> runCommand("Previous", NativeBridge.previous()));
        playPauseButton.setOnClickListener(view -> runCommand("Play/Pause", NativeBridge.playPause()));
        nextButton.setOnClickListener(view -> runCommand("Next", NativeBridge.next()));
        transferButton.setOnClickListener(view -> runCommand("Transfer", NativeBridge.transfer()));

        positionSeekBar.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
            @Override
            public void onProgressChanged(SeekBar seekBar, int progress, boolean fromUser) {
                if (fromUser) {
                    String left = formatTime(progress);
                    String right = formatTime(Math.max(latestSnapshot.durationMs, 0));
                    timeText.setText(left + " / " + right);
                }
            }

            @Override
            public void onStartTrackingTouch(SeekBar seekBar) {
                userSeeking = true;
            }

            @Override
            public void onStopTrackingTouch(SeekBar seekBar) {
                userSeeking = false;
                runCommand("Seek", NativeBridge.seekTo(seekBar.getProgress()));
            }
        });

        volumeSeekBar.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
            @Override
            public void onProgressChanged(SeekBar seekBar, int progress, boolean fromUser) {
                if (fromUser) {
                    volumeValueText.setText(progress + "%");
                }
            }

            @Override
            public void onStartTrackingTouch(SeekBar seekBar) {
                userAdjustingVolume = true;
            }

            @Override
            public void onStopTrackingTouch(SeekBar seekBar) {
                userAdjustingVolume = false;
                int volume = (int) Math.round((seekBar.getProgress() / 100.0) * 65535.0);
                runCommand("Volume", NativeBridge.setVolume(volume));
            }
        });
    }

    private void runCommand(String action, boolean success) {
        if (!success) {
            showNativeError(action + " command failed");
        }
        refreshUi();
    }

    private void refreshUi() {
        latestSnapshot = PlaybackSnapshot.fromJson(NativeBridge.snapshotJson());

        statusText.setText(latestSnapshot.statusMessage);
        connectionText.setText(
                String.format(
                        Locale.US,
                        "device=%s ready=%s connected=%s state=%s",
                        latestSnapshot.deviceName.isEmpty() ? "unknown" : latestSnapshot.deviceName,
                        latestSnapshot.ready ? "yes" : "no",
                        latestSnapshot.connected ? "yes" : "no",
                        latestSnapshot.playbackStateLabel()));

        titleText.setText(nonEmptyOrFallback(latestSnapshot.title, getString(R.string.track_unknown)));
        artistText.setText(nonEmptyOrFallback(latestSnapshot.artist, getString(R.string.artist_unknown)));
        albumText.setText(nonEmptyOrFallback(latestSnapshot.album, getString(R.string.album_unknown)));

        playPauseButton.setText(
                latestSnapshot.playbackState == PlaybackSnapshot.STATE_PLAYING
                        ? R.string.button_pause
                        : R.string.button_play);

        int duration = Math.max(latestSnapshot.durationMs, 1);
        int position = Math.max(0, Math.min(latestSnapshot.positionMs, duration));

        if (!userSeeking) {
            positionSeekBar.setMax(duration);
            positionSeekBar.setProgress(position);
        }

        timeText.setText(formatTime(position) + " / " + formatTime(latestSnapshot.durationMs));

        int volumePercent = (int) Math.round((latestSnapshot.volume / 65535.0) * 100.0);
        volumePercent = Math.max(0, Math.min(volumePercent, 100));

        if (!userAdjustingVolume) {
            volumeSeekBar.setProgress(volumePercent);
        }
        volumeValueText.setText(volumePercent + "%");

        updateArtwork(latestSnapshot.artworkUrl);

        String backgroundError = NativeBridge.takeLastError();
        if (backgroundError != null && !backgroundError.trim().isEmpty()) {
            Toast.makeText(this, backgroundError, Toast.LENGTH_SHORT).show();
        }
    }

    private void updateArtwork(String artworkUrl) {
        if (artworkUrl == null || artworkUrl.trim().isEmpty()) {
            currentArtworkUrl = "";
            artworkImage.setImageResource(android.R.drawable.ic_menu_gallery);
            return;
        }

        if (artworkUrl.equals(currentArtworkUrl)) {
            return;
        }

        currentArtworkUrl = artworkUrl;
        artworkImage.setImageResource(android.R.drawable.ic_menu_gallery);

        artworkExecutor.execute(() -> {
            Bitmap bitmap = downloadBitmap(artworkUrl);
            runOnUiThread(() -> {
                if (!artworkUrl.equals(currentArtworkUrl)) {
                    return;
                }
                if (bitmap != null) {
                    artworkImage.setImageBitmap(bitmap);
                } else {
                    artworkImage.setImageResource(android.R.drawable.ic_menu_gallery);
                }
            });
        });
    }

    private Bitmap downloadBitmap(String artworkUrl) {
        HttpURLConnection connection = null;
        InputStream input = null;
        try {
            URL url = new URL(artworkUrl);
            connection = (HttpURLConnection) url.openConnection();
            connection.setConnectTimeout(5000);
            connection.setReadTimeout(5000);
            connection.setInstanceFollowRedirects(true);

            input = connection.getInputStream();
            return BitmapFactory.decodeStream(input);
        } catch (IOException ignored) {
            return null;
        } finally {
            if (input != null) {
                try {
                    input.close();
                } catch (IOException ignored) {
                    // no-op
                }
            }
            if (connection != null) {
                connection.disconnect();
            }
        }
    }

    private String nonEmptyOrFallback(String value, String fallback) {
        if (value == null || value.trim().isEmpty()) {
            return fallback;
        }
        return value;
    }

    private String formatTime(int millis) {
        int safeMillis = Math.max(0, millis);
        int totalSeconds = safeMillis / 1000;
        int hours = totalSeconds / 3600;
        int minutes = (totalSeconds % 3600) / 60;
        int seconds = totalSeconds % 60;

        if (hours > 0) {
            return String.format(Locale.US, "%d:%02d:%02d", hours, minutes, seconds);
        }
        return String.format(Locale.US, "%02d:%02d", minutes, seconds);
    }

    private void showNativeError(String fallback) {
        String message = NativeBridge.takeLastError();
        if (message == null || message.trim().isEmpty()) {
            message = fallback;
        }
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show();
    }
}
